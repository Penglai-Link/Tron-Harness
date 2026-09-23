mod git;
mod office;

use super::coding_tools::{json_observation, resolve_optional_directory, resolve_path};
use crate::{
    application::resource::ResourceService,
    domain::agent::{AgentRunContext, Tool, ToolError, ToolExecutionResult, ToolRequest},
};
use serde_json::{Value, json};
use std::{
    io::Read,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::Arc,
    time::{Duration, Instant},
};

const MAX_FILE_BYTES: u64 = 32 * 1024 * 1024;
const MAX_OUTPUT_BYTES: usize = 65536;

pub(super) fn tools(resources: ResourceService) -> Vec<Arc<dyn Tool>> {
    [
        "worktree_list",
        "worktree_create",
        "worktree_select",
        "git_status",
        "git_diff",
        "git_log",
        "git_switch_branch",
        "word_read",
        "word_create",
        "word_replace_text",
        "excel_read",
        "excel_write",
    ]
    .into_iter()
    .map(|name| {
        Arc::new(ProductivityTool {
            name,
            resources: resources.clone(),
        }) as Arc<dyn Tool>
    })
    .collect()
}
struct ProductivityTool {
    name: &'static str,
    resources: ResourceService,
}
impl Tool for ProductivityTool {
    fn name(&self) -> &str {
        self.name
    }
    fn exec(
        &self,
        request: ToolRequest,
        context: &AgentRunContext,
    ) -> Result<ToolExecutionResult, ToolError> {
        if request.input.to_string().len() > 4 * 1024 * 1024 {
            return Err(error("tool input exceeds 4 MiB"));
        }
        let definition = crate::application::agent::BusinessToolCatalog::tool(
            &self.name.replacen('_', ".", 1),
            &context.locale,
        )
        .ok_or_else(|| error("tool definition unavailable"))?;
        let validator = jsonschema::validator_for(&definition.input_schema).map_err(error)?;
        if let Err(failure) = validator.validate(&request.input) {
            return Err(error(format!("invalid tool input: {failure}")));
        }
        if ["worktree_create", "worktree_select"].contains(&self.name)
            && context.execution_scope.applications.is_empty()
        {
            return Err(error(
                "worktree selection requires a project-linked application",
            ));
        }
        let result = if self.name.starts_with("word_") || self.name.starts_with("excel_") {
            office::execute(self.name, &request.input, context, &self.resources)?
        } else {
            git::execute(self.name, &request.input, context)?
        };
        if (self.name.starts_with("word_")
            || self.name.starts_with("excel_")
            || self.name == "worktree_list")
            && result.to_string().len() > MAX_OUTPUT_BYTES
        {
            return Err(error("result exceeds 64 KiB; request a smaller range"));
        }
        json_observation(self.name, &result)
    }
}
fn error(message: impl ToString) -> ToolError {
    ToolError::ExecutionFailed(message.to_string())
}
fn text<'a>(input: &'a Value, key: &str) -> Result<&'a str, ToolError> {
    input
        .get(key)
        .and_then(Value::as_str)
        .filter(|v| !v.trim().is_empty())
        .ok_or_else(|| error(format!("{key} is required")))
}
fn path(input: &Value, key: &str, context: &AgentRunContext) -> Result<PathBuf, ToolError> {
    resolve_path(context, text(input, key)?)
}
fn cwd(input: &Value, context: &AgentRunContext) -> Result<PathBuf, ToolError> {
    resolve_optional_directory(context, input.get("cwd").and_then(Value::as_str))?
        .ok_or_else(|| error("working directory is required"))
}
fn read_file(path: &Path) -> Result<Vec<u8>, ToolError> {
    let file = std::fs::File::open(path).map_err(error)?;
    if file.metadata().map_err(error)?.len() > MAX_FILE_BYTES {
        return Err(error("file exceeds 32 MiB limit"));
    }
    let mut bytes = Vec::new();
    file.take(MAX_FILE_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(error)?;
    if bytes.len() as u64 > MAX_FILE_BYTES {
        return Err(error("file exceeds 32 MiB limit"));
    }
    Ok(bytes)
}
fn write_new(path: &Path, bytes: &[u8]) -> Result<(), ToolError> {
    let parent = path
        .parent()
        .ok_or_else(|| error("output parent is required"))?;
    if !parent.is_dir() {
        return Err(error("output parent directory does not exist"));
    }
    let mut temporary = tempfile::NamedTempFile::new_in(parent).map_err(error)?;
    std::io::Write::write_all(&mut temporary, bytes).map_err(error)?;
    temporary.persist_noclobber(path).map_err(error)?;
    Ok(())
}
fn run(command: &str, args: &[String], directory: &Path, timeout: u64) -> Result<Value, ToolError> {
    let mut cmd = Command::new(command);
    if command == "git" {
        for (key, _) in std::env::vars_os() {
            if key.to_string_lossy().starts_with("GIT_") {
                cmd.env_remove(key);
            }
        }
        cmd.env("GIT_OPTIONAL_LOCKS", "0")
            .env("GIT_TERMINAL_PROMPT", "0");
    }
    let mut child = cmd
        .args(args)
        .current_dir(directory)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(error)?;
    fn drain(mut stream: impl Read + Send + 'static) -> std::thread::JoinHandle<(Vec<u8>, bool)> {
        std::thread::spawn(move || {
            let mut output = Vec::new();
            let mut truncated = false;
            let mut buf = [0u8; 8192];
            loop {
                match stream.read(&mut buf) {
                    Ok(0) | Err(_) => break,
                    Ok(n) => {
                        let remaining = MAX_OUTPUT_BYTES.saturating_sub(output.len());
                        output.extend_from_slice(&buf[..n.min(remaining)]);
                        truncated |= n > remaining;
                    }
                }
            }
            (output, truncated)
        })
    }
    let stdout = drain(
        child
            .stdout
            .take()
            .ok_or_else(|| error("stdout unavailable"))?,
    );
    let stderr = drain(
        child
            .stderr
            .take()
            .ok_or_else(|| error("stderr unavailable"))?,
    );
    let deadline = Instant::now() + Duration::from_secs(timeout);
    let status = loop {
        if let Some(status) = child.try_wait().map_err(error)? {
            break status;
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            return Err(error(
                "command timed out; inspect the working tree before retrying",
            ));
        }
        std::thread::sleep(Duration::from_millis(20));
    };
    // Drain threads can outlive an inherited pipe; do not wait indefinitely for descendants.
    let drain_deadline = Instant::now() + Duration::from_secs(1);
    while (!stdout.is_finished() || !stderr.is_finished()) && Instant::now() < drain_deadline {
        std::thread::sleep(Duration::from_millis(10));
    }
    if !stdout.is_finished() || !stderr.is_finished() {
        return Err(error("command descendants kept output pipes open"));
    }
    let (out, out_truncated) = stdout.join().map_err(|_| error("stdout reader failed"))?;
    let (err, err_truncated) = stderr.join().map_err(|_| error("stderr reader failed"))?;
    Ok(
        json!({"cwd":directory,"exitCode":status.code(),"stdout":String::from_utf8_lossy(&out),"stderr":String::from_utf8_lossy(&err),"truncated":out_truncated||err_truncated}),
    )
}
