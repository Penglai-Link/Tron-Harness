//! Safe local Office chart conversion. Isolated profile, no network, bounded output.
use crate::application::knowledge_base::{
    KnowledgeBaseError, office_visuals::validate_office_chart_input,
    parsing::KnowledgeOfficeRenderer,
};
use async_trait::async_trait;
use std::{
    path::{Path, PathBuf},
    process::Stdio,
    time::Duration,
};
#[derive(Clone)]
pub struct LocalOfficeChartRenderer;
fn invalid(key: &'static str) -> KnowledgeBaseError {
    KnowledgeBaseError::invalid(key)
}
fn invalid_detail(error: impl std::fmt::Display) -> KnowledgeBaseError {
    KnowledgeBaseError::from_detail(error)
}
#[async_trait]
impl KnowledgeOfficeRenderer for LocalOfficeChartRenderer {
    async fn render_pdf(
        &self,
        filename: &str,
        bytes: &[u8],
    ) -> Result<Vec<u8>, KnowledgeBaseError> {
        let plan = validate_office_chart_input(filename, bytes)?;
        if !plan.has_charts() {
            return Err(invalid("integration.office.no_native_charts"));
        }
        #[cfg(target_os = "macos")]
        {
            local_mac(&plan.format, bytes).await
        }
        #[cfg(not(target_os = "macos"))]
        {
            Err(invalid("integration.office.local_render_macos_only"))
        }
    }
}
#[cfg(target_os = "macos")]
async fn local_mac(format: &str, bytes: &[u8]) -> Result<Vec<u8>, KnowledgeBaseError> {
    const EXEC: &str = "/Applications/LibreOffice.app/Contents/MacOS/soffice";
    if !Path::new(EXEC).is_file() || !Path::new("/usr/bin/sandbox-exec").is_file() {
        return Err(invalid("integration.office.runtime_missing"));
    }
    let temporary = tempfile::Builder::new()
        .prefix("tron-office-chart-")
        .tempdir()
        .map_err(invalid_detail)?;
    let job = temporary.path().canonicalize().map_err(invalid_detail)?;
    let profile = job.join("profile");
    let out = job.join("out");
    tokio::fs::create_dir_all(profile.join("user"))
        .await
        .map_err(invalid_detail)?;
    tokio::fs::create_dir(&out).await.map_err(invalid_detail)?;
    // Installed LO schema: DisableMacrosExecution overrides Basic/Python/JS security.
    // Calc ScLkUpdMode LM_NEVER = 1. Static validation rejects ALL external relationships.
    tokio::fs::write(profile.join("user/registrymodifications.xcu"),r#"<?xml version="1.0"?><oor:items xmlns:oor="http://openoffice.org/2001/registry"><item oor:path="/org.openoffice.Office.Common/Security/Scripting"><prop oor:name="DisableMacrosExecution" oor:op="fuse"><value>true</value></prop><prop oor:name="MacroSecurityLevel" oor:op="fuse"><value>3</value></prop></item><item oor:path="/org.openoffice.Office.Calc/Content/Update"><prop oor:name="Link" oor:op="fuse"><value>1</value></prop></item></oor:items>"#).await.map_err(invalid_detail)?;
    let input = job.join(format!("input.{format}"));
    tokio::fs::write(&input, bytes)
        .await
        .map_err(invalid_detail)?;
    let quoted =
        |path: &Path| serde_json::to_string(&path.to_string_lossy()).map_err(invalid_detail);
    // Only the job directory is writable/readable user data. Network and other user files
    // are denied independently of document/config settings. No shell or broad subprocesses.
    let sandbox = format!(
        r#"(version 1)
(deny default)
(allow process-fork)
(allow process-exec (literal {exec}))
(allow sysctl-read)
(allow mach-lookup)
(allow mach-register)
(allow process-info* (target self))
(allow signal (target self))
(allow ipc-posix*)
(allow file-read-metadata)
(allow file-read* (subpath "/Applications/LibreOffice.app") (subpath "/System") (subpath "/Library/Fonts") (subpath "/usr/lib") (subpath "/private/var/db/dyld") (subpath "/usr/share") (subpath {job}) (subpath "/dev"))
(allow file-write* (subpath {job}) (literal "/dev/null"))
(deny network*)
"#,
        exec = quoted(Path::new(EXEC))?,
        job = quoted(&job)?
    );
    let sandbox_path = job.join("sandbox.sb");
    tokio::fs::write(&sandbox_path, sandbox)
        .await
        .map_err(invalid_detail)?;
    let filter = if format == "xlsx" {
        r#"pdf:calc_pdf_Export:{"SinglePageSheets":{"type":"boolean","value":"true"}}"#
    } else {
        "pdf:writer_pdf_Export"
    };
    let profile_url = url::Url::from_directory_path(&profile)
        .map_err(|_| invalid("integration.office.isolation_path_invalid"))?;
    let mut command = tokio::process::Command::new("/usr/bin/sandbox-exec");
    command
        .arg("-f")
        .arg(&sandbox_path)
        .arg(EXEC)
        .arg(format!("-env:UserInstallation={profile_url}"))
        .args([
            "--headless",
            "--nologo",
            "--nodefault",
            "--nofirststartwizard",
            "--norestore",
            "--convert-to",
        ])
        .arg(filter)
        .arg("--outdir")
        .arg(&out)
        .arg(&input)
        .current_dir(&job)
        .env("TMPDIR", &job)
        .env("SAL_USE_VCLPLUGIN", "svp")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .kill_on_drop(true);
    let diagnostics = std::env::var("TRON_OFFICE_RENDER_TEST_DIAGNOSTICS").as_deref() == Ok("1");
    if diagnostics {
        command.stderr(Stdio::from(
            std::fs::File::create(job.join("stderr.log")).map_err(invalid_detail)?,
        ));
    }
    let mut child = command.spawn().map_err(invalid_detail)?;
    let status = tokio::time::timeout(Duration::from_secs(45), child.wait())
        .await
        .map_err(|_| invalid("integration.office.conversion_timeout"))?
        .map_err(invalid_detail)?;
    if !status.success() {
        if diagnostics {
            use std::io::Read;
            let mut text = String::new();
            if let Ok(file) = std::fs::File::open(job.join("stderr.log")) {
                let _ = file.take(4096).read_to_string(&mut text);
            }
            tracing::warn!("Synthetic Office sandbox diagnostics: {text}");
        }
        return Err(invalid_detail(format!(
            "sandbox conversion exited with status {status}"
        )));
    }
    let pdf: PathBuf = out.join("input.pdf");
    let size = tokio::fs::metadata(&pdf)
        .await
        .map_err(|_| invalid("integration.office.pdf_missing"))?
        .len();
    if size == 0 || size > 20 * 1024 * 1024 {
        return Err(invalid("integration.office.output_too_large"));
    }
    let bytes = tokio::fs::read(pdf).await.map_err(invalid_detail)?;
    if !bytes.starts_with(b"%PDF-") || bytes.len() > 20 * 1024 * 1024 {
        return Err(invalid("integration.office.output_invalid"));
    }
    Ok(bytes)
}
