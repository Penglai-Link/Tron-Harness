//! Desktop control belongs to the two primary controllers, never their delegates.
use super::agent::ToolCatalogRecord;

pub const PACK_ID: &str = "tron.computer-use";
pub const DOMAIN: &str = "tron.computer-use";
pub const SKILL: &str = "cua-driver";
pub const CONTROLLERS: [&str; 2] = ["main-agent", "main-dude"];

pub fn allows(agent: &str) -> bool {
    CONTROLLERS
        .iter()
        .any(|code| code.eq_ignore_ascii_case(agent))
}

pub fn is_tool(id: &str, name: &str, domain: Option<&str>, pack: Option<&str>) -> bool {
    pack == Some(PACK_ID)
        || domain == Some(DOMAIN)
        || domain == Some("mcp.tron.computer-use.cua-driver")
        || id.starts_with("tron.computer-use.")
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
    pack == Some(PACK_ID) || name == SKILL
}

pub fn skill_allowed(agent: &str, name: &str, pack: Option<&str>) -> bool {
    allows(agent) || !is_skill(name, pack)
}
