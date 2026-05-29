mod fake_store;

use std::sync::{Arc, Mutex};

pub(super) use fake_store::{FakeStore, StoreCall};

use crate::{
    application::graph::OntologyScopeResolver,
    domain::graph::{
        GraphScopeCommand, GraphStorageDimensions, OntologyClass, OntologyConstraint,
        OntologyDomain, OntologyDomainResource, OntologyError, OntologyProperty,
        OntologyQueryLimit, OntologyRangeResource, OntologyRangeTarget, OntologyResourceType,
        OntologySearchQuery, OntologyShaclShape, OntologyShapeProperty, ResolvedGraphScope,
    },
};

#[derive(Clone)]
pub(super) struct FakeScopeResolver {
    scope: ResolvedGraphScope,
    calls: Arc<Mutex<Vec<GraphScopeCommand>>>,
}

impl FakeScopeResolver {
    pub(super) fn new(scope: ResolvedGraphScope) -> Self {
        Self {
            scope,
            calls: Arc::new(Mutex::new(Vec::new())),
        }
    }
}

impl OntologyScopeResolver for FakeScopeResolver {
    fn resolve_ontology_scope(
        &self,
        command: GraphScopeCommand,
    ) -> Result<ResolvedGraphScope, OntologyError> {
        self.calls
            .lock()
            .expect("resolver calls mutex poisoned")
            .push(command);
        Ok(self.scope.clone())
    }
}

pub(super) fn scope_command() -> GraphScopeCommand {
    GraphScopeCommand {
        namespace: "ontology".to_owned(),
        workspace_code: "WS-CHILD".to_owned(),
        app_code: Some("APP-A".to_owned()),
        user_code: Some("USER-A".to_owned()),
    }
}

pub(super) fn resolved_scope() -> ResolvedGraphScope {
    ResolvedGraphScope {
        schema: "ontology".to_owned(),
        named_graph: "WS-ROOT".to_owned(),
        root_workspace_code: "WS-ROOT".to_owned(),
        current_workspace_code: "WS-CHILD".to_owned(),
        visible_workspace_codes: vec!["WS-ROOT".to_owned(), "WS-CHILD".to_owned()],
        storage_dimensions: GraphStorageDimensions {
            namespace: "ontology".to_owned(),
            workspace_code: "WS-CHILD".to_owned(),
            app_code: "APP-A".to_owned(),
            user_code: "USER-A".to_owned(),
        },
    }
}

pub(super) fn class_resource(class_code: &str) -> OntologyClass {
    OntologyClass {
        class_code: class_code.to_owned(),
        ontology_domain: OntologyDomain::Operations,
        label: "Task".to_owned(),
        description: Some("task class".to_owned()),
        parent_class_codes: vec!["ENTITY".to_owned()],
    }
}

pub(super) fn domain_resource(domain_code: &str) -> OntologyDomainResource {
    OntologyDomainResource {
        domain_code: domain_code.to_owned(),
        ontology_domain: OntologyDomain::Operations,
        label: "task domain".to_owned(),
        description: Some("task domain resource".to_owned()),
        class_code: "TASK".to_owned(),
    }
}

pub(super) fn range_resource(range_code: &str) -> OntologyRangeResource {
    OntologyRangeResource {
        range_code: range_code.to_owned(),
        ontology_domain: OntologyDomain::Operations,
        label: "user range".to_owned(),
        description: Some("user range resource".to_owned()),
        target: OntologyRangeTarget::Class {
            class_code: "USER".to_owned(),
        },
    }
}

pub(super) fn property_resource(property_code: &str) -> OntologyProperty {
    OntologyProperty {
        property_code: property_code.to_owned(),
        ontology_domain: OntologyDomain::Operations,
        label: "owner".to_owned(),
        description: Some("owner property".to_owned()),
        domain_codes: vec!["TASK_DOMAIN".to_owned()],
        range_codes: vec!["USER_RANGE".to_owned()],
    }
}

pub(super) fn constraint_resource(constraint_code: &str) -> OntologyConstraint {
    OntologyConstraint {
        constraint_code: constraint_code.to_owned(),
        ontology_domain: OntologyDomain::Operations,
        label: "required owner".to_owned(),
        description: Some("owner is required".to_owned()),
        required: true,
        min_count: Some(1),
        max_count: Some(1),
        allowed_range_codes: vec!["USER_RANGE".to_owned()],
        pattern: None,
    }
}

pub(super) fn shape_resource(shape_code: &str) -> OntologyShaclShape {
    OntologyShaclShape {
        shape_code: shape_code.to_owned(),
        ontology_domain: OntologyDomain::Operations,
        label: "task shape".to_owned(),
        description: Some("task constraints".to_owned()),
        target_class_code: "TASK".to_owned(),
        property_constraints: vec![OntologyShapeProperty {
            property_code: "OWNER".to_owned(),
            constraint_codes: vec!["OWNER_REQUIRED".to_owned()],
        }],
    }
}

pub(super) fn search_query() -> OntologySearchQuery {
    OntologySearchQuery {
        resource_types: vec![
            OntologyResourceType::Class,
            OntologyResourceType::Domain,
            OntologyResourceType::Range,
            OntologyResourceType::Shape,
        ],
        ontology_domains: vec![
            OntologyDomain::Operations,
            OntologyDomain::Custom("custom_ops".to_owned()),
        ],
        codes: vec![
            "TASK".to_owned(),
            "TASK_DOMAIN".to_owned(),
            "USER_RANGE".to_owned(),
        ],
        label: Some("task".to_owned()),
        target_class_code: Some("TASK".to_owned()),
        limit: OntologyQueryLimit::new(10).expect("limit should be valid"),
    }
}
