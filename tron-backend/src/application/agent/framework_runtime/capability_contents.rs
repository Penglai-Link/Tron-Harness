use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CapabilityContentInfo {
    pub kind: String,
    pub name: String,
    pub description: String,
}

fn content(
    kind: &str,
    name: impl Into<String>,
    description: impl Into<String>,
) -> CapabilityContentInfo {
    CapabilityContentInfo {
        kind: kind.into(),
        name: name.into(),
        description: description.into(),
    }
}

pub(super) fn manifest_contents(
    manifest: &eliteforge_agent_capability_runtime::CapabilityPackManifest,
) -> Vec<CapabilityContentInfo> {
    let mut items = Vec::new();
    for (kind, names) in [
        ("Plugin", &manifest.plugins),
        ("Skill", &manifest.skills),
        ("Resource", &manifest.resources),
        ("Permission", &manifest.permissions),
    ] {
        items.extend(names.iter().map(|name| content(kind, name, "")));
    }
    items.extend(
        manifest
            .pages
            .iter()
            .map(|page| content("Page", &page.id, &page.title)),
    );
    items.extend(manifest.mcp.iter().map(|mcp| content("Mcp", &mcp.id, "")));
    items.extend(
        manifest
            .extension_points
            .iter()
            .map(|point| content("Extension", &point.id, point.version.to_string())),
    );
    items.extend(manifest.implementations.iter().map(|implementation| {
        content(
            "Implementation",
            &implementation.id,
            &implementation.extension,
        )
    }));
    items.extend(
        manifest.dependencies.iter().map(|dependency| {
            content("Dependency", &dependency.id, dependency.version.to_string())
        }),
    );
    items
}

pub(super) fn builtin_contents(id: &str) -> Vec<CapabilityContentInfo> {
    use eliteforge_agent_official_plugin_tools_agents::*;
    let tools = match id {
        "framework.local.scoped" => {
            eliteforge_agent_official_plugin_tools_local::local_tool_catalog()
        }
        "framework.tool-pack.agent-collaboration" => vec![
            AgentSpawnTool::descriptor(),
            ThreadCreateTool::descriptor(),
            ThreadSendTool::descriptor(),
            ThreadAnswerTool::descriptor(),
            ThreadWaitTool::descriptor(),
            ThreadReadTool::descriptor(),
            ThreadListTool::descriptor(),
            ThreadCloseTool::descriptor(),
        ],
        "framework.a2ui" => {
            let mut tools = eliteforge_agent_official_plugin_a2ui::tool_descriptors();
            tools.push(eliteforge_agent_official_plugin_a2ui::expand_descriptor());
            tools
        }
        "framework.personai" => eliteforge_agent_capability_personai::personai_tool_catalog(),
        "framework.web" => vec![eliteforge_agent_official_plugin_tools_web::web_fetch_descriptor()],
        "framework.media" => eliteforge_agent_official_plugin_tools_media::media_tool_catalog(),
        _ => vec![],
    };
    let mut items: Vec<_> = tools
        .into_iter()
        .map(|tool| content("Tool", tool.call_name, tool.description))
        .collect();
    if id == eliteforge_agent_official_plugin_a2ui::A2UI_PACK_ID {
        items.push(content(
            "Skill",
            eliteforge_agent_official_plugin_a2ui::A2UI_SKILL_NAME,
            "",
        ));
    }
    if id == "framework.external-agent" {
        items.push(content(
            "Runtime",
            "Independent ExternalAgentLoop",
            "Independent external execution with native session ownership and observation events",
        ));
    }
    if id == "tron.harness" {
        items.push(content(
            "Context",
            "TronContext",
            tron_agent_harness::CONTEXT_IMPLEMENTATION_DESCRIPTION,
        ));
        items.push(content(
            "Policy",
            "Tron tool permissions",
            tron_agent_harness::TOOL_IMPLEMENTATION_DESCRIPTION,
        ));
        items.extend(
            crate::application::agent::BusinessToolCatalog::list_tools(None, None, "en-US")
                .into_iter()
                .filter(|tool| super::super::tools::official_id(&tool.tool_name).is_none())
                .map(|tool| content("Tool", tool.tool_name, tool.description)),
        );
    }
    items
}

#[cfg(test)]
mod content_tests {
    use super::*;

    #[test]
    fn builtin_contents_are_complete_and_keep_official_tools_out_of_harness() {
        let packs = super::super::capability_pack_catalog();
        assert!(packs.iter().all(|pack| !pack.contents.is_empty()));
        let a2ui = packs
            .iter()
            .find(|pack| pack.id == eliteforge_agent_official_plugin_a2ui::A2UI_PACK_ID)
            .unwrap();
        assert!(a2ui.contents.iter().any(|item| item.kind == "Skill"
            && item.name == eliteforge_agent_official_plugin_a2ui::A2UI_SKILL_NAME));
        let harness = packs.iter().find(|pack| pack.id == "tron.harness").unwrap();
        assert!(
            !harness
                .contents
                .iter()
                .any(|item| item.name == "web_fetch" || item.name.starts_with("personai_"))
        );
        let collaboration = packs
            .iter()
            .find(|pack| pack.id == eliteforge_agent_capability_tools::AGENT_COLLABORATION_PACK)
            .unwrap();
        assert_eq!(collaboration.contents.len(), 8);
    }

    #[test]
    fn imported_contents_include_manifest_contributions_without_activation() {
        let manifest = serde_json::from_value(serde_json::json!({
            "manifest_version": 1, "id": "test.pack", "name": "Test", "version": "1.0.0", "framework": "*",
            "plugins": ["test.plugin"], "skills": ["skills/demo"], "resources": ["assets/icon.png"],
            "permissions": ["network"], "dependencies": [{"id": "test.base", "version": "^1"}],
            "pages": [{"id": "test.page", "title": "Demo", "plugin": "test.plugin", "entry": "index.html"}]
        })).unwrap();
        let items = manifest_contents(&manifest);
        assert_eq!(items.len(), 6);
        for kind in [
            "Plugin",
            "Skill",
            "Resource",
            "Permission",
            "Dependency",
            "Page",
        ] {
            assert!(items.iter().any(|item| item.kind == kind));
        }
        assert_eq!(serde_json::to_value(&items).unwrap()[0]["kind"], "Plugin");
    }
}
