use serde::Serialize;

#[path = "capability_skill_contents.rs"]
mod skills;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CapabilityContentInfo {
    pub kind: String,
    pub name: String,
    pub display_name: String,
    pub description: String,
    pub provides: Vec<String>,
}

fn content(
    kind: &str,
    name: impl Into<String>,
    description: impl Into<String>,
) -> CapabilityContentInfo {
    let name=name.into();
    CapabilityContentInfo {
        kind: kind.into(),
        display_name: name.clone(),
        name,
        description: description.into(),
        provides: Vec::new(),
    }
}

pub(super) fn manifest_contents(
    manifest: &eliteforge_agent_capability_runtime::CapabilityPackManifest,
) -> Vec<CapabilityContentInfo> {
    let mut items = Vec::new();
    items.extend(manifest.tools.iter().map(|entry| content("Tool",&entry.descriptor.call_name,&entry.descriptor.description)));
    for (kind, names) in [
        ("Plugin", &manifest.plugins),
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

pub(super) fn installed_contents(
    path: &std::path::Path,
    manifest: &eliteforge_agent_capability_runtime::CapabilityPackManifest,
    locale: &str,
) -> Result<Vec<CapabilityContentInfo>, String> {
    let mut items = manifest_contents(manifest);
    items.extend(skills::skill_contents(path, manifest)?);
    items.extend(navigation_contents(path, manifest, locale)?);
    Ok(items)
}

pub(super) fn builtin_tool_descriptors(id: &str) -> Vec<eliteforge_agent_core::ToolDescriptor> {
    use eliteforge_agent_official_plugin_tools_agents::*;
    match id {
        "framework.local.scoped" => {
            eliteforge_agent_official_plugin_tools_local::local_tool_catalog()
        }
        "framework.tool-pack.agent-collaboration" => vec![
            AskCallerTool::descriptor(),
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
        "framework.a2ui" => eliteforge_agent_official_plugin_a2ui::tool_descriptors(),
        "framework.personai" => eliteforge_agent_capability_personai::personai_tool_catalog(),
        "framework.web" => vec![eliteforge_agent_official_plugin_tools_web::web_read_descriptor()],
        "framework.media" => eliteforge_agent_official_plugin_tools_media::media_tool_catalog(),
        tron_harness_capability::PACK_ID => {
            eliteforge_agent_core::ToolProvider::catalog_metadata(&super::super::tools::HarnessTools)
                .unwrap_or_default()
                .into_iter()
                .map(|tool| tool.descriptor)
                .collect()
        }
        _ => vec![],
    }
}

pub(super) fn builtin_contents(id: &str) -> Vec<CapabilityContentInfo> {
    let mut items: Vec<_> = builtin_tool_descriptors(id)
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
    if id == eliteforge_agent_capability_development::PACK_ID {
        items.extend(
            eliteforge_agent_capability_development::skill_names()
                .into_iter()
                .map(|name| content("Skill", name, "")),
        );
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
        "framework.personai" => &["capability.personai"],
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
            &[
                eliteforge_agent_core::TOOL_SEARCH_CALL_NAME,
                eliteforge_agent_core::TOOL_EXECUTE_CALL_NAME,
            ],
        ),
        "framework.web.tools" => (
            "Fetch text from public webpages",
            &["WebToolsPlugin", "web_read"],
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
    if let Some(name) = plugin_display_name(id, "en-US") {
        item.display_name = name.into();
    }
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

pub(super) fn plugin_display_name(id: &str, locale: &str) -> Option<&'static str> {
    let (english, chinese) = match id {
        "framework.external-agent" => ("External Agent Binding", "外部 Agent 绑定"),
        "framework.loop" => ("External Agent Loop", "外部 Agent 执行循环"),
        "framework.pty.service" => ("Scoped Terminal Service", "授权终端服务"),
        "framework.local.scoped.tools" => ("Scoped Local Tools", "授权本地工具"),
        "tools.agents" => ("Agent Collaboration Tools", "Agent 协作工具"),
        "agent.a2ui" => ("A2UI Interfaces", "A2UI 交互界面"),
        "capability.personai" => ("PersonAI Services", "PersonAI 服务"),
        "framework.tools.discovery" => ("Tool Discovery", "工具发现"),
        "framework.web.tools" => ("Web Tools", "网页工具"),
        "framework.media.tools" => ("Media Tools", "媒体工具"),
        "tron.harness.tools" => ("Tron Business Tools", "Tron 业务工具"),
        "tron.harness.tool-permissions" => ("Tron Tool Permissions", "Tron 工具权限"),
        "framework.context-assembler" => ("Context Assembly", "上下文组装"),
        _ => return None,
    };
    Some(if locale.starts_with("zh") { chinese } else { english })
}


pub(super) fn navigation_contents(path: &std::path::Path, manifest: &eliteforge_agent_capability_runtime::CapabilityPackManifest, locale: &str) -> Result<Vec<CapabilityContentInfo>, String> {
    let navigation = super::super::host::host_navigation::read_navigation(path, manifest)?;
    let mut items = Vec::new();
    for page in super::super::host::host_pages::read_pages(path, manifest)? {
        let title = page.title.get(if locale.starts_with("zh") { "zh-CN" } else { "en-US" }).cloned().unwrap_or_else(|| page.id.clone());
        let mut item = content("Page", &page.id, "");
        item.display_name = title;
        items.push(item);
    }
    for domain in &navigation.domains {
        let mut item = content("NavigationDomain", &domain.code, domain.description.localized().value(locale));
        item.display_name = domain.name.localized().value(locale).into();
        items.push(item);
    }
    for menu in &navigation.menus {
        let mut item = content("NavigationMenu", &menu.code, menu.description.localized().value(locale));
        item.display_name = menu.name.localized().value(locale).into();
        items.push(item);
    }
    for (domain, layout) in &navigation.layouts {
        let mut item = content("NavigationLayout", domain, "");
        item.provides = layout.iter().map(|item| item.menu_code.clone()).collect();
        items.push(item);
    }
    Ok(items)
}
