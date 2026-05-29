use crate::{
    application::workspace::WorkspaceService,
    domain::{
        graph::{GraphScope, GraphScopeCommand, GraphScopeError, ResolvedGraphScope},
        workspace::{WorkspaceDetail, WorkspaceError, WorkspaceStatus},
    },
};

pub trait WorkspaceDetailProvider {
    fn get_workspace(&self, workspace_code: &str) -> Result<WorkspaceDetail, WorkspaceError>;
}

impl WorkspaceDetailProvider for WorkspaceService {
    fn get_workspace(&self, workspace_code: &str) -> Result<WorkspaceDetail, WorkspaceError> {
        WorkspaceService::get_workspace(self, workspace_code)
    }
}

#[derive(Debug, Clone)]
pub struct ScopeResolver<W = WorkspaceService> {
    workspaces: W,
}

impl ScopeResolver<WorkspaceService> {
    pub fn new(workspaces: WorkspaceService) -> Self {
        Self { workspaces }
    }
}

impl<W> ScopeResolver<W>
where
    W: WorkspaceDetailProvider,
{
    pub fn with_workspace_provider(workspaces: W) -> Self {
        Self { workspaces }
    }

    pub fn resolve(
        &self,
        command: GraphScopeCommand,
    ) -> Result<ResolvedGraphScope, GraphScopeError> {
        let scope = GraphScope::from_command(command)?;
        let workspace = self
            .workspaces
            .get_workspace(&scope.workspace_code)
            .map_err(|_| GraphScopeError::WorkspaceResolutionFailed)?;
        if workspace.status != WorkspaceStatus::Active {
            return Err(GraphScopeError::ArchivedWorkspace);
        }

        crate::domain::graph::resolve_storage_scope(scope, &workspace.path)
    }
}

#[cfg(test)]
mod tests {
    use std::{cell::RefCell, collections::HashMap, rc::Rc};

    use crate::{
        application::graph::{ScopeResolver, WorkspaceDetailProvider},
        domain::{
            graph::{
                GraphScope, GraphScopeCommand, GraphScopeError, GraphStorageProperty,
                STORAGE_ABSENT_SENTINEL, ensure_not_reserved_storage_property,
            },
            workspace::{
                WorkspaceDetail, WorkspaceError, WorkspaceLineageItem, WorkspaceStatus,
                WorkspaceTreeNode,
            },
        },
    };

    #[derive(Debug, Clone)]
    enum WorkspaceOutcome {
        Found(WorkspaceDetail),
        Failed(WorkspaceError),
    }

    #[derive(Debug, Clone)]
    struct FakeWorkspaceProvider {
        workspaces: HashMap<String, WorkspaceOutcome>,
        calls: Rc<RefCell<Vec<String>>>,
    }

    impl FakeWorkspaceProvider {
        fn new(workspaces: impl IntoIterator<Item = (String, WorkspaceOutcome)>) -> Self {
            Self {
                workspaces: workspaces.into_iter().collect(),
                calls: Rc::new(RefCell::new(Vec::new())),
            }
        }

        fn calls(&self) -> Vec<String> {
            self.calls.borrow().clone()
        }
    }

    impl WorkspaceDetailProvider for FakeWorkspaceProvider {
        fn get_workspace(&self, workspace_code: &str) -> Result<WorkspaceDetail, WorkspaceError> {
            self.calls.borrow_mut().push(workspace_code.to_owned());
            match self.workspaces.get(workspace_code) {
                Some(WorkspaceOutcome::Found(detail)) => Ok(detail.clone()),
                Some(WorkspaceOutcome::Failed(error)) => Err(error.clone()),
                None => Err(WorkspaceError::NotFound),
            }
        }
    }

    #[test]
    fn graph_scope_maps_namespace_to_schema_and_root_workspace_to_named_graph() {
        let workspaces = FakeWorkspaceProvider::new([(
            "WS-CHILD".to_owned(),
            WorkspaceOutcome::Found(workspace_detail(
                "WS-CHILD",
                WorkspaceStatus::Active,
                lineage(&["WS-ROOT", "WS-CHILD"]),
            )),
        )]);
        let resolver = ScopeResolver::with_workspace_provider(workspaces);

        let resolved = resolver
            .resolve(command(
                "knowledge_graph",
                "WS-CHILD",
                Some("APP-A"),
                Some("USER-A"),
            ))
            .expect("scope should resolve");

        assert_eq!(resolved.schema, "knowledge_graph");
        assert_eq!(resolved.named_graph, "WS-ROOT");
        assert_eq!(resolved.root_workspace_code, "WS-ROOT");
        assert_eq!(resolved.current_workspace_code, "WS-CHILD");
    }

    #[test]
    fn child_workspace_reads_only_include_root_to_current_visible_lineage() {
        let workspaces = FakeWorkspaceProvider::new([(
            "WS-LEAF".to_owned(),
            WorkspaceOutcome::Found(workspace_detail(
                "WS-LEAF",
                WorkspaceStatus::Active,
                lineage(&["WS-ROOT", "WS-CHILD", "WS-LEAF"]),
            )),
        )]);
        let resolver = ScopeResolver::with_workspace_provider(workspaces);

        let resolved = resolver
            .resolve(command("ontology", "WS-LEAF", None, None))
            .expect("scope should resolve");

        assert_eq!(
            resolved.visible_workspace_codes,
            vec!["WS-ROOT", "WS-CHILD", "WS-LEAF"]
        );
        assert!(
            !resolved
                .visible_workspace_codes
                .contains(&"WS-SIBLING".to_owned())
        );
    }

    #[test]
    fn write_dimensions_store_current_workspace_while_named_graph_stays_root() {
        let workspaces = FakeWorkspaceProvider::new([(
            "WS-CHILD".to_owned(),
            WorkspaceOutcome::Found(workspace_detail(
                "WS-CHILD",
                WorkspaceStatus::Active,
                lineage(&["WS-ROOT", "WS-CHILD"]),
            )),
        )]);
        let resolver = ScopeResolver::with_workspace_provider(workspaces);

        let resolved = resolver
            .resolve(command(
                "knowledge_graph",
                "WS-CHILD",
                Some("APP-A"),
                Some("USER-A"),
            ))
            .expect("scope should resolve");

        assert_eq!(resolved.named_graph, "WS-ROOT");
        assert_eq!(resolved.storage_dimensions.workspace_code, "WS-CHILD");
        assert_eq!(resolved.storage_dimensions.namespace, "knowledge_graph");
        assert_eq!(resolved.storage_dimensions.app_code, "APP-A");
        assert_eq!(resolved.storage_dimensions.user_code, "USER-A");
    }

    #[test]
    fn missing_app_and_user_normalize_to_storage_sentinel_only_in_storage_dimensions() {
        let graph_scope =
            GraphScope::from_command(command("knowledge_graph", "WS-ROOT", None, None))
                .expect("command scope should remain valid with missing dimensions");
        assert_eq!(graph_scope.app_code, None);
        assert_eq!(graph_scope.user_code, None);

        let workspaces = FakeWorkspaceProvider::new([(
            "WS-ROOT".to_owned(),
            WorkspaceOutcome::Found(workspace_detail(
                "WS-ROOT",
                WorkspaceStatus::Active,
                lineage(&["WS-ROOT"]),
            )),
        )]);
        let resolver = ScopeResolver::with_workspace_provider(workspaces);

        let resolved = resolver
            .resolve(command("knowledge_graph", "WS-ROOT", Some("  "), None))
            .expect("scope should resolve");

        assert_eq!(
            resolved.storage_dimensions.app_code,
            STORAGE_ABSENT_SENTINEL
        );
        assert_eq!(
            resolved.storage_dimensions.user_code,
            STORAGE_ABSENT_SENTINEL
        );
    }

    #[test]
    fn caller_supplied_sentinel_and_reserved_storage_properties_are_rejected() {
        let app_error = GraphScope::from_command(command(
            "knowledge_graph",
            "WS-ROOT",
            Some(STORAGE_ABSENT_SENTINEL),
            None,
        ))
        .expect_err("explicit sentinel app code should be rejected");
        assert_eq!(app_error, GraphScopeError::ReservedSentinelValue);

        let user_error = GraphScope::from_command(command(
            "knowledge_graph",
            "WS-ROOT",
            None,
            Some(STORAGE_ABSENT_SENTINEL),
        ))
        .expect_err("explicit sentinel user code should be rejected");
        assert_eq!(user_error, GraphScopeError::ReservedSentinelValue);

        assert_eq!(
            ensure_not_reserved_storage_property(GraphStorageProperty::WorkspaceCode.as_str()),
            Err(GraphScopeError::ReservedPropertyName)
        );
        assert_eq!(ensure_not_reserved_storage_property("displayName"), Ok(()));
    }

    #[test]
    fn invalid_scope_inputs_return_typed_errors_before_workspace_lookup() {
        let workspaces = FakeWorkspaceProvider::new([]);
        let resolver = ScopeResolver::with_workspace_provider(workspaces.clone());

        assert_eq!(
            resolver.resolve(command("", "WS-ROOT", None, None)),
            Err(GraphScopeError::InvalidNamespace)
        );
        assert_eq!(workspaces.calls(), Vec::<String>::new());

        assert_eq!(
            resolver.resolve(command("KnowledgeGraph", "WS-ROOT", None, None)),
            Err(GraphScopeError::InvalidNamespace)
        );
        assert_eq!(
            resolver.resolve(command("knowledge_graph", " ", None, None)),
            Err(GraphScopeError::InvalidWorkspaceCode)
        );
        assert_eq!(
            resolver.resolve(command("knowledge_graph", "ws-root", None, None)),
            Err(GraphScopeError::InvalidWorkspaceCode)
        );
        assert_eq!(workspaces.calls(), Vec::<String>::new());
    }

    #[test]
    fn archived_workspace_empty_lineage_and_lookup_failures_return_typed_errors() {
        let workspaces = FakeWorkspaceProvider::new([
            (
                "WS-ARCHIVED".to_owned(),
                WorkspaceOutcome::Found(workspace_detail(
                    "WS-ARCHIVED",
                    WorkspaceStatus::Archived,
                    lineage(&["WS-ROOT", "WS-ARCHIVED"]),
                )),
            ),
            (
                "WS-EMPTY".to_owned(),
                WorkspaceOutcome::Found(workspace_detail(
                    "WS-EMPTY",
                    WorkspaceStatus::Active,
                    Vec::new(),
                )),
            ),
            (
                "WS-MISSING".to_owned(),
                WorkspaceOutcome::Failed(WorkspaceError::NotFound),
            ),
        ]);
        let resolver = ScopeResolver::with_workspace_provider(workspaces);

        assert_eq!(
            resolver.resolve(command("knowledge_graph", "WS-ARCHIVED", None, None)),
            Err(GraphScopeError::ArchivedWorkspace)
        );
        assert_eq!(
            resolver.resolve(command("knowledge_graph", "WS-EMPTY", None, None)),
            Err(GraphScopeError::EmptyWorkspaceLineage)
        );
        assert_eq!(
            resolver.resolve(command("knowledge_graph", "WS-MISSING", None, None)),
            Err(GraphScopeError::WorkspaceResolutionFailed)
        );
    }

    fn command(
        namespace: &str,
        workspace_code: &str,
        app_code: Option<&str>,
        user_code: Option<&str>,
    ) -> GraphScopeCommand {
        GraphScopeCommand {
            namespace: namespace.to_owned(),
            workspace_code: workspace_code.to_owned(),
            app_code: app_code.map(str::to_owned),
            user_code: user_code.map(str::to_owned),
        }
    }

    fn lineage(codes: &[&str]) -> Vec<WorkspaceLineageItem> {
        codes
            .iter()
            .enumerate()
            .map(|(depth, code)| WorkspaceLineageItem {
                workspace_code: (*code).to_owned(),
                name: format!("{code} name"),
                depth,
            })
            .collect()
    }

    fn workspace_detail(
        workspace_code: &str,
        status: WorkspaceStatus,
        path: Vec<WorkspaceLineageItem>,
    ) -> WorkspaceDetail {
        WorkspaceDetail {
            id: "1".to_owned(),
            workspace_code: workspace_code.to_owned(),
            name: format!("{workspace_code} name"),
            description: None,
            parent_workspace_code: None,
            depth: path.len(),
            status,
            path,
            children: Vec::<WorkspaceTreeNode>::new(),
            direct_user_count: 0,
            role_grant_count: 0,
            effective_user_count: 0,
            created_at: "2026-05-29T00:00:00Z".to_owned(),
            updated_at: "2026-05-29T00:00:00Z".to_owned(),
            archived_at: (status == WorkspaceStatus::Archived)
                .then(|| "2026-05-29T00:00:00Z".to_owned()),
        }
    }
}
