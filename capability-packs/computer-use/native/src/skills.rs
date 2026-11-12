use eliteforge_agent_capability_runtime::RpcFault;
use eliteforge_agent_core::{SkillCatalog, SkillDetail, SkillFile, SkillResource, SkillSummary};
use serde_json::Value;

// Only the public, compiled-in resource accessors are reused. No MCP server is
// started and no protocol requests are passed to the upstream MCP adapter.
fn files() -> &'static [(&'static str, &'static str)] {
    cua_driver_core::mcp_skills::embedded_files()
}

fn summary() -> Result<SkillSummary, RpcFault> {
    let markdown = source("SKILL.md")?;
    let yaml = markdown
        .strip_prefix("---\n")
        .and_then(|body| body.split_once("\n---"))
        .ok_or_else(|| RpcFault::new("invalid official skill frontmatter"))?
        .0;
    let value: Value =
        serde_yaml::from_str(yaml).map_err(|error| RpcFault::new(error.to_string()))?;
    Ok(SkillSummary {
        name: "cua-driver".into(),
        description: value["description"].as_str().unwrap_or_default().into(),
        source: "tron.computer-use".into(),
        location: "skill://cua-driver/SKILL.md".into(),
        license: value["license"].as_str().map(str::to_owned),
        compatibility: value["compatibility"].as_str().map(str::to_owned),
        // Framework's metadata contract is string-valued. Preserve nested
        // vendor metadata as JSON here, while load returns the original bytes.
        metadata: value["metadata"]
            .as_object()
            .into_iter()
            .flatten()
            .map(|(key, value)| {
                (
                    key.clone(),
                    value
                        .as_str()
                        .map(str::to_owned)
                        .unwrap_or_else(|| value.to_string()),
                )
            })
            .collect(),
        allowed_tools: value["allowed-tools"].as_str().map(str::to_owned),
    })
}

fn source(name: &str) -> Result<&'static str, RpcFault> {
    files()
        .iter()
        .find_map(|(file, text)| (*file == name).then_some(*text))
        .ok_or_else(|| RpcFault::new("unknown official skill resource"))
}

pub fn call(operation: &str, params: &Value) -> Result<Value, RpcFault> {
    if operation == "release" {
        return Ok(Value::Null);
    }
    if operation == "catalog" {
        return encode(SkillCatalog {
            skills: vec![summary()?],
            diagnostics: vec![],
        });
    }
    if params["name"] != "cua-driver" {
        return Err(RpcFault::new("unknown skill"));
    }
    match operation {
        "load" => encode(SkillDetail {
            skill: summary()?,
            skill_markdown: source("SKILL.md")?.into(),
            files: files()
                .iter()
                .filter(|(name, _)| *name != "SKILL.md")
                .map(|(name, text)| SkillFile {
                    path: (*name).into(),
                    size_bytes: text.len() as u64,
                })
                .collect(),
        }),
        "read_resource" => {
            let path = params["path"]
                .as_str()
                .ok_or_else(|| RpcFault::new("missing resource path"))?;
            if path == "SKILL.md" {
                return Err(RpcFault::new("load skill instructions with skill_load"));
            }
            encode(SkillResource {
                skill_name: "cua-driver".into(),
                path: path.into(),
                content: source(path)?.into(),
            })
        }
        _ => Err(RpcFault::new("unknown skill operation")),
    }
}

fn encode(value: impl serde::Serialize) -> Result<Value, RpcFault> {
    serde_json::to_value(value).map_err(|error| RpcFault::new(error.to_string()))
}
