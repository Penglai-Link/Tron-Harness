fn build_workspace_service(database_url: &str, event_bus: EventBusHandle) -> WorkspaceService {
    let repository = SqliteWorkspaceRepository::connect(database_url)
        .expect("workspace SQLite repository should initialize");
    WorkspaceService::with_event_bus(repository, event_bus)
}

fn build_app_service(database_url: &str, workspaces: WorkspaceService) -> AppService {
    let repository = SqliteAppRepository::connect(database_url)
        .expect("app SQLite repository should initialize");
    AppService::new(repository, workspaces)
}

fn build_project_service(
    database_url: &str,
    workspaces: WorkspaceService,
    apps: AppService,
) -> ProjectService {
    let repository = SqliteProjectRepository::connect(database_url)
        .expect("project SQLite repository should initialize");
    ProjectService::new(repository, workspaces, apps).with_directory_inspector(Arc::new(
        crate::adapters::outbound::application_directory::GitApplicationDirectoryInspector,
    ))
}

fn build_task_service(
    database_url: &str,
    workspaces: WorkspaceService,
    apps: AppService,
    event_bus: EventBusHandle,
) -> TaskService {
    let repository = SqliteTaskRepository::connect(database_url)
        .expect("task SQLite repository should initialize");
    TaskService::with_event_bus(repository, workspaces, apps, event_bus)
}

fn build_automation_service(
    database_url: &str,
    workspaces: WorkspaceService,
    event_bus: EventBusHandle,
) -> AutomationService {
    let repository = SqliteAutomationRepository::connect(database_url)
        .expect("automation SQLite repository should initialize");
    AutomationService::with_event_bus(repository, workspaces, event_bus)
}

fn build_variable_service(
    database_url: &str,
    workspaces: WorkspaceService,
    config: &AppConfig,
) -> VariableService {
    let repository = SqliteVariableRepository::connect(database_url)
        .expect("variable SQLite repository should initialize");
    let cipher = VariableCipherProvider::from_config(config.variable_crypto.clone());
    VariableService::new(repository, workspaces, cipher)
}

fn build_resource_service(
    database_url: &str,
    workspaces: WorkspaceService,
    variables: VariableService,
    config: &AppConfig,
    event_bus: EventBusHandle,
) -> ResourceService {
    let repository = SqliteResourceRepository::connect(database_url)
        .expect("resource SQLite repository should initialize");
    let storage_profiles = config
        .resource
        .storage
        .profiles
        .iter()
        .map(|(profile_code, profile)| {
            let storage: Arc<dyn ObjectStorage> = match profile {
                ResourceStorageProfile::Local(profile) => Arc::new(
                    LocalObjectStorage::new(&profile.root)
                        .expect("local resource storage should initialize"),
                ),
                ResourceStorageProfile::S3(profile) => Arc::new(
                    S3ObjectStorage::from_config(profile)
                        .expect("s3 resource storage should initialize"),
                ),
            };
            (profile_code.clone(), storage)
        })
        .collect::<BTreeMap<_, _>>();
    ResourceService::with_event_bus(
        repository,
        workspaces,
        variables,
        config.resource.clone(),
        storage_profiles,
        event_bus,
        Arc::new(ProtocolResourceConnectionVerifier::new(
            Duration::from_millis(config.resource.connection_verification_timeout_ms),
        )),
    )
    .expect("resource service should initialize")
    .with_profile_scope()
}

fn build_memory_work_domain_service(database_url: &str) -> MemoryWorkDomainService {
    let repository = SqliteMemoryWorkDomainRepository::connect(database_url)
        .expect("memory work domain SQLite repository should initialize");
    MemoryWorkDomainService::new(repository)
}

fn build_navigation_service(database_url: &str, max_depth: usize) -> NavigationService {
    let repository = SqliteNavigationRepository::connect(database_url)
        .expect("navigation SQLite repository should initialize");
    NavigationService::new(repository, max_depth)
}

fn build_mcp_service(
    database_url: &str,
    workspaces: WorkspaceService,
    variables: VariableService,
    runtime_http_client: reqwest::Client,
) -> McpService {
    let repository = SqliteMcpRepository::connect(database_url)
        .expect("MCP SQLite repository should initialize");
    McpService::new(repository, workspaces, variables, runtime_http_client)
}

fn build_skill_registry_service(
    database_url: &str,
    workspaces: WorkspaceService,
    registry_root: &str,
) -> SkillRegistryService {
    let repository = SqliteSkillRegistryRepository::connect(database_url)
        .expect("skill registry SQLite repository should initialize");
    SkillRegistryService::new(repository, workspaces, registry_root)
}

fn build_model_service(
    database_url: &str,
    workspaces: WorkspaceService,
    variables: VariableService,
) -> ModelService {
    let repository = SqliteModelRepository::connect(database_url)
        .expect("model SQLite repository should initialize");
    ModelService::new(repository, workspaces, variables.cipher())
}

fn build_model_provider_service(
    database_url: &str,
    variables: VariableService,
) -> crate::application::model_provider::ModelProviderService {
    crate::application::model_provider::ModelProviderService::connect(
        database_url,
        variables.cipher(),
    )
    .expect("model provider SQLite repository should initialize")
}

fn build_prompt_service(
    database_url: &str,
    workspaces: WorkspaceService,
    variables: VariableService,
    id_provider: Arc<dyn IdGenerator>,
) -> PromptService {
    let repository = SqlitePromptRepository::connect(database_url)
        .expect("prompt SQLite repository should initialize");
    let service = PromptService::new(repository, workspaces, variables, id_provider);
    service
        .retire_removed_builtins()
        .expect("retired bundled prompts should be removed");
    if should_seed_builtin_prompts(database_url) {
        service
            .bootstrap_builtins()
            .expect("bundled prompt catalog should initialize");
    }
    service
}

fn should_seed_builtin_prompts(database_url: &str) -> bool {
    match std::env::var("TRON_PROMPT_SEED_BUILTINS") {
        Ok(value) if value.eq_ignore_ascii_case("false") || value == "0" => false,
        Ok(_) => true,
        Err(_) => {
            let path = std::path::Path::new(database_url);
            database_url != ":memory:"
                && !database_url.starts_with("file:")
                && !path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| name.starts_with("tron-"))
        }
    }
}

fn build_agent_management_service(
    database_url: &str,
    workspaces: WorkspaceService,
    system_agents: &crate::config::SystemAgentConfig,
) -> AgentManagementService {
    let repository = SqliteAgentManagementRepository::connect(database_url)
        .expect("agent management SQLite repository should initialize");
    AgentManagementService::new(repository, workspaces, system_agents)
}
