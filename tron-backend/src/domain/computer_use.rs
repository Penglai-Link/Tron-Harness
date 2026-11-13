//! Desktop control belongs to the two primary controllers, never their delegates.
use super::agent::ToolCatalogRecord;

pub const PACK_ID: &str = "tron.computer-use";
pub const PACK_ID_PREFIX: &str = "tron.computer-use.";
pub const DOMAIN: &str = "tron.computer-use";
pub const SKILL: &str = "cua-driver";
pub const CONTROLLERS: [&str; 2] = ["main-agent", "main-dude"];

/// The legacy single pack and every per-platform variant share these rules.
pub fn is_pack(pack: &str) -> bool {
    pack == PACK_ID
        || pack
            .strip_prefix(PACK_ID_PREFIX)
            .is_some_and(|suffix| !suffix.is_empty())
}

pub fn allows(agent: &str) -> bool {
    CONTROLLERS
        .iter()
        .any(|code| code.eq_ignore_ascii_case(agent))
}

pub fn is_tool(id: &str, name: &str, domain: Option<&str>, pack: Option<&str>) -> bool {
    pack.is_some_and(is_pack)
        || domain == Some(DOMAIN)
        || domain == Some("mcp.tron.computer-use.cua-driver")
        || id.starts_with(PACK_ID_PREFIX)
        || name.starts_with("computer_")
        || name.starts_with("mcp_tron_computer-use_cua-driver_")
}

pub fn tool_allowed(agent: &str, tool: &ToolCatalogRecord) -> bool {
    allows(agent)
        || !is_tool(
            &tool.tool_code,
            &tool.tool_name,
            Some(&tool.domain_code),
            tool.source.pack_id.as_deref(),
        )
}

pub fn is_skill(name: &str, pack: Option<&str>) -> bool {
    pack.is_some_and(is_pack) || name == SKILL
}

pub fn skill_allowed(agent: &str, name: &str, pack: Option<&str>) -> bool {
    allows(agent) || !is_skill(name, pack)
}
