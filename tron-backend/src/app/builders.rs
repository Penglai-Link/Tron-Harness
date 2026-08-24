fn build_user_service(config: &AppConfig, event_bus: EventBusHandle) -> UserService {
    let repository = SqliteUserRepository::connect(&config.database_url)
        .expect("user SQLite repository should initialize");
    UserService::with_event_bus(
        repository,
        BcryptPasswordHasher,
        config.auth.clone(),
        event_bus,
    )
}

fn build_role_service(database_url: &str, event_bus: EventBusHandle) -> RoleService {
    let repository = SqliteRoleRepository::connect(database_url)
        .expect("role SQLite repository should initialize");
    RoleService::with_event_bus(repository, event_bus)
}

fn build_workspace_service(
    database_url: &str,
    users: UserService,
    roles: RoleService,
    event_bus: EventBusHandle,
) -> WorkspaceService {
    let repository = SqliteWorkspaceRepository::connect(database_url)
        .expect("workspace SQLite repository should initialize");
    WorkspaceService::with_event_bus(repository, users, roles, event_bus)
}

fn build_app_service(database_url: &str, workspaces: WorkspaceService) -> AppService {
    let repository = SqliteAppRepository::connect(database_url)
        .expect("app SQLite repository should initialize");
    AppService::new(repository, workspaces)
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

fn build_chat_service_with_id_provider<I>(database_url: &str, id_provider: Arc<I>) -> ChatService
where
    I: SnowflakeIdProvider + 'static,
{
    let repository = SqliteChatRepository::connect(database_url)
        .expect("chat SQLite repository should initialize");
    ChatService::with_id_provider(repository, id_provider)
}

fn build_short_term_memory_service_with_id_provider<I>(
    database_url: &str,
    config: &AppConfig,
    id_provider: Arc<I>,
) -> ShortTermMemoryService
where
    I: SnowflakeIdProvider + 'static,
{
    let repository = SqliteShortTermMemoryRepository::connect(database_url)
        .expect("short-term memory SQLite repository should initialize");
    let service = ShortTermMemoryService::with_id_provider(
        repository,
        config.short_term_memory.clone(),
        id_provider,
    );
    service
        .recover_goal_state_on_startup()
        .expect("short-term memory goal recovery should initialize");
    if let Err(error) = service.cleanup_event_log_on_startup() {
        eprintln!("short-term memory event log cleanup skipped: {error:?}");
    }
    if let Err(error) = service.sweep_externalized_tool_output_orphans() {
        eprintln!("short-term memory tool output orphan sweep skipped: {error:?}");
    }
    service
}

fn build_variable_service(
    database_url: &str,
    workspaces: WorkspaceService,
    config: &AppConfig,
) -> VariableService {
    let repository = SqliteVariableRepository::connect(database_url)
        .expect("variable SQLite repository should initialize");
    let cipher = Sm4GcmVariableCipher::from_config(config.variable_crypto.clone())
        .expect("variable crypto should initialize");
    VariableService::new(repository, workspaces, cipher)
}

fn build_resource_service(
    database_url: &str,
    workspaces: WorkspaceService,
    roles: RoleService,
    users: UserService,
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
            let storage: Arc<dyn ObjectStorage> = Arc::new(
                LocalObjectStorage::new(&profile.root)
                    .expect("local resource storage should initialize"),
            );
            (profile_code.clone(), storage)
        })
        .collect::<BTreeMap<_, _>>();
    ResourceService::with_event_bus(
        repository,
        workspaces,
        roles,
        users,
        variables,
        config.resource.clone(),
        storage_profiles,
        event_bus,
        Arc::new(ProtocolResourceConnectionVerifier::new(Duration::from_millis(
            config.resource.connection_verification_timeout_ms,
        ))),
    )
    .expect("resource service should initialize")
}

fn build_memory_work_domain_service(database_url: &str) -> MemoryWorkDomainService {
    let repository = SqliteMemoryWorkDomainRepository::connect(database_url)
        .expect("memory work domain SQLite repository should initialize");
    MemoryWorkDomainService::new(repository)
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

fn build_model_service(database_url: &str, workspaces: WorkspaceService) -> ModelService {
    let repository = SqliteModelRepository::connect(database_url)
        .expect("model SQLite repository should initialize");
    ModelService::new(repository, workspaces)
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

fn build_open_app_service(
    database_url: &str,
    users: UserService,
    jwt_secret: &str,
) -> OpenAppService {
    let repository = SqliteOpenAppRepository::connect(database_url)
        .expect("open-app SQLite repository should initialize");
    OpenAppService::new(repository, users, Aes256SecretBox::new(jwt_secret))
}

fn build_agent_run_history_service(database_url: &str) -> AgentRunHistoryService {
    let repository = SqliteAgentRunRepository::connect(database_url)
        .expect("agent run SQLite repository should initialize");
    AgentRunHistoryService::new(repository)
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
