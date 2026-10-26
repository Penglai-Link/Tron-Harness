use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CapabilityContentInfo {
    pub kind: String,
    pub name: String,
    pub description: String,
    pub provides: Vec<String>,
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
        provides: Vec::new(),
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
            AgentGetTool::descriptor(),
            AgentSpawnTool::descriptor(),
            ChildWaitTool::descriptor(),
            ChildSendTool::descriptor(),
            ChildAnswerTool::descriptor(),
            ChildCancelTool::descriptor(),
            ChildReadTool::descriptor(),
            ChildListTool::descriptor(),
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
    if id == tron_harness_capability::PACK_ID {
        items.extend(
            tron_harness_capability::skill_names()
                .into_iter()
                .map(|name| content("Skill", name, "")),
        );
        items.push(content(
            "Context",
            "TronContext",
            tron_harness_capability::CONTEXT_IMPLEMENTATION_DESCRIPTION,
        ));
        items.push(content(
            "Policy",
            "Tron tool permissions",
            tron_harness_capability::TOOL_IMPLEMENTATION_DESCRIPTION,
        ));
        items.extend(
            crate::application::agent::BusinessToolCatalog::list_tools(None, None, "en-US")
                .into_iter()
                .filter(|tool| super::super::tools::official_id(&tool.tool_name).is_none())
                .map(|tool| content("Tool", tool.tool_name, tool.description)),
        );
    }
    // Keep detail contributions and the extension catalog on the same source.
    items.extend(
        super::pack_extension_implementations()
            .into_iter()
            .filter(|implementation| implementation.provider_pack_id == id)
            .map(|implementation| {
                content(
                    "Implementation",
                    implementation.implementation,
                    format!(
                        "{}: {}",
                        implementation.extension_point, implementation.description
                    ),
                )
            }),
    );
    let plugins: &[&str] = match id {
        "framework.external-agent" => &["framework.external-agent", "framework.loop"],
        "framework.local.scoped" => &["framework.pty.service", "framework.local.scoped.tools"],
        "framework.tool-pack.agent-collaboration" => &["tools.agents"],
        "framework.a2ui" => &["agent.a2ui"],
        "framework.personai" => &["capability.personai", "framework.tools.discovery"],
        "framework.web" => &["framework.web.tools"],
        "framework.media" => &["framework.media.tools"],
        "tron.harness" => &[
            "tron.harness.tools",
            "tron.harness.tool-permissions",
            "framework.context-assembler",
        ],
        _ => &[],
    };
    let resources: &[(&str, &str)] = match id {
        "framework.a2ui" => &[
            (
                "A2uiService",
                "Shared surface state and interaction service",
            ),
            ("references/examples.json", "A2UI skill examples"),
            ("catalog.json", "Negotiated component catalog index"),
            ("protocol.json", "A2UI server-to-client protocol schema"),
            ("common-types.json", "Shared A2UI schema types"),
            (
                "components/agent/",
                "Per-component schemas for the negotiated Agent catalog",
            ),
            (
                "components/minimal/",
                "Per-component schemas for the negotiated minimal catalog",
            ),
        ],
        "framework.local.scoped" => &[("PtyService", "Shared scoped PTY sessions")],
        "framework.personai" => &[
            ("PersonAiConnection", "Account-bound PersonAI connection"),
            ("PersonAiServices", "PersonAI API clients"),
            ("PersonAiModels", "PersonAI model catalog"),
            ("PersonAiModelSync", "Account-bound model synchronization"),
            ("ModelRuntimeService", "PersonAI model runtime"),
            (
                "ModelCatalogRefreshService",
                "PersonAI model catalog synchronization",
            ),
        ],
        _ => &[],
    };
    items.extend(
        resources
            .iter()
            .map(|(name, description)| content("Resource", *name, *description)),
    );
    let plugin_items: Vec<_> = plugins
        .iter()
        .map(|plugin| plugin_content(plugin, &items))
        .collect();
    items.extend(plugin_items);
    items
}

fn plugin_content(id: &str, contents: &[CapabilityContentInfo]) -> CapabilityContentInfo {
    let (description, provides): (&str, &[&str]) = match id {
        "framework.external-agent" => (
            "Connect external agents to host execution and session bridging",
            &["HostExternalAgent"],
        ),
        "framework.loop" => (
            "Run external agents with native session ownership and observation events",
            &["Independent ExternalAgentLoop"],
        ),
        "framework.pty.service" => (
            "Manage scoped interactive terminal sessions",
            &["PtyService"],
        ),
        "framework.local.scoped.tools" => (
            "Provide filesystem and command tools within the authorized scope",
            &["ScopedLocalToolsPlugin"],
        ),
        "tools.agents" => (
            "Delegate to child agents and manage independent threads",
            &["AgentToolsPlugin"],
        ),
        "agent.a2ui" => (
            "Publish interactive interfaces with shared state, guidance and resources",
            &["A2uiService", "A2UI Guidance", "a2ui-interface"],
        ),
        "capability.personai" => (
            "Provide account-bound PersonAI tools, clients and model services",
            &[
                "PersonAiToolProvider",
                "PersonAiConnection",
                "PersonAiServices",
                "PersonAiModels",
                "PersonAiModelSync",
                "ModelRuntimeService",
                "ModelCatalogRefreshService",
            ],
        ),
        "framework.tools.discovery" => (
            "Discover and execute available tools on demand",
            &[eliteforge_agent_core::TOOL_SEARCH_CALL_NAME, eliteforge_agent_core::TOOL_EXECUTE_CALL_NAME],
        ),
        "framework.web.tools" => (
            "Fetch text from public webpages",
            &["WebToolsPlugin", "web_fetch"],
        ),
        "framework.media.tools" => (
            "Read images and PDF files within the authorized scope",
            &["MediaToolsPlugin"],
        ),
        "tron.harness.tools" => ("Provide Tron business tools", &["Tron HarnessTools"]),
        "tron.harness.tool-permissions" => (
            "Apply platform tool permissions and risk approval",
            &["TronToolPolicy"],
        ),
        "framework.context-assembler" => (
            "Assemble bounded context for agent execution",
            &["TronContext + bounded context preparation"],
        ),
        _ => ("", &[]),
    };
    let mut item = content("Plugin", id, description);
    item.provides = provides.iter().map(|name| (*name).into()).collect();
    let provides_tools = matches!(
        id,
        "framework.local.scoped.tools"
            | "tools.agents"
            | "agent.a2ui"
            | "capability.personai"
            | "framework.web.tools"
            | "framework.media.tools"
            | "tron.harness.tools"
    );
    for entry in contents {
        if (provides_tools && entry.kind == "Tool")
            || (id == "agent.a2ui" && matches!(entry.kind.as_str(), "Resource" | "Skill"))
        {
            if !item.provides.contains(&entry.name) {
                item.provides.push(entry.name.clone());
            }
        }
    }
    item
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
        for name in [
            "tron-theme-spec",
            "eliteforge-memory-spec",
            "eliteforge-memory-dreaming",
        ] {
            assert!(
                harness
                    .contents
                    .iter()
                    .any(|item| item.kind == "Skill" && item.name == name)
            );
        }
        let collaboration = packs
            .iter()
            .find(|pack| pack.id == eliteforge_agent_capability_tools::AGENT_COLLABORATION_PACK)
            .unwrap();
        assert_eq!(
            collaboration
                .contents
                .iter()
                .filter(|item| item.kind == "Tool")
                .count(),
            15
        );
    }

    #[test]
    fn builtin_details_include_owned_extensions_and_plugins() {
        let packs = super::super::capability_pack_catalog();
        let implementations = super::super::pack_extension_implementations();
        for pack in &packs {
            for implementation in implementations
                .iter()
                .filter(|item| item.provider_pack_id == pack.id)
            {
                assert!(
                    pack.contents
                        .iter()
                        .any(|item| item.kind == "Implementation"
                            && item.name == implementation.implementation
                            && item
                                .description
                                .starts_with(&format!("{}:", implementation.extension_point))),
                    "{} is missing {}",
                    pack.id,
                    implementation.implementation
                );
            }
        }
        let a2ui = packs
            .iter()
            .find(|pack| pack.id == "framework.a2ui")
            .unwrap();
        for kind in ["Tool", "Skill", "Plugin", "Implementation", "Resource"] {
            assert!(a2ui.contents.iter().any(|item| item.kind == kind));
        }
        assert_eq!(
            a2ui.contents
                .iter()
                .filter(|item| item.kind == "Implementation")
                .count(),
            3
        );
        let local = packs
            .iter()
            .find(|pack| pack.id == "framework.local.scoped")
            .unwrap();
        assert!(
            !local
                .contents
                .iter()
                .any(|item| item.name.contains("Harness"))
        );
        let harness = packs.iter().find(|pack| pack.id == "tron.harness").unwrap();
        assert!(
            harness
                .contents
                .iter()
                .any(|item| item.name == "Tron HarnessTools")
        );
    }

    #[test]
    fn builtin_plugins_declare_descriptions_and_provided_capabilities() {
        let packs = super::super::capability_pack_catalog();
        for pack in packs {
            for plugin in pack.contents.iter().filter(|item| item.kind == "Plugin") {
                assert!(!plugin.description.is_empty(), "{}", plugin.name);
                assert!(!plugin.provides.is_empty(), "{}", plugin.name);
            }
        }
        let external = builtin_contents("framework.external-agent");
        let plugin = external
            .iter()
            .find(|item| item.name == "framework.loop")
            .unwrap();
        assert_eq!(plugin.provides, ["Independent ExternalAgentLoop"]);
        let json = serde_json::to_value(&external).unwrap();
        assert!(
            json.as_array()
                .unwrap()
                .iter()
                .all(|item| item["provides"].is_array())
        );
    }

    #[test]
    fn imported_contents_include_manifest_contributions_without_activation() {
        let manifest = serde_json::from_value(serde_json::json!({
            "manifest_version": 1, "id": "test.pack", "name": "Test", "version": "1.0.0", "framework": "*",
            "plugins": ["test.plugin"], "skills": ["skills/demo"], "resources": ["assets/icon.png"],
            "permissions": ["network"], "dependencies": [{"id": "test.base", "version": "^1"}],
            "pages": [{"id": "test.page", "title": "Demo", "plugin": "test.plugin", "entry": "index.html"}],
            "extension_points": [{"id": "test.extension", "owner": "test.pack", "version": "1.0.0", "category": "custom", "multiplicity": "multiple"}],
            "implementations": [{"id": "test.implementation", "extension": "test.extension", "contract": "^1"}],
            "mcp": [{"id": "test.mcp", "transport": "http", "url": "https://example.com/mcp"}]
        })).unwrap();
        let items = manifest_contents(&manifest);
        assert_eq!(items.len(), 9);
        for kind in [
            "Plugin",
            "Skill",
            "Resource",
            "Permission",
            "Dependency",
            "Page",
            "Extension",
            "Implementation",
            "Mcp",
        ] {
            assert!(items.iter().any(|item| item.kind == kind));
        }
        assert_eq!(serde_json::to_value(&items).unwrap()[0]["kind"], "Plugin");
    }
}
