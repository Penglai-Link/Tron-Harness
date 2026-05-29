use crate::{
    application::graph::{
        OntologyService,
        ontology_test_support::{
            FakeScopeResolver, FakeStore, StoreCall, class_resource, constraint_resource,
            domain_resource, property_resource, range_resource, resolved_scope, scope_command,
            search_query, shape_resource,
        },
    },
    domain::graph::{
        OntologyConstraint, OntologyDomain, OntologyDomainResource, OntologyError,
        OntologyProperty, OntologyRangeResource, OntologyRangeTarget, OntologySearchQuery,
        OntologyShaclShape, OntologyShapeProperty,
    },
};

fn store_with_valid_resources() -> FakeStore {
    FakeStore::with_resources(
        &["ENTITY", "TASK", "USER"],
        &["OWNER"],
        &["TASK_DOMAIN"],
        &["USER_RANGE"],
        &["OWNER_REQUIRED"],
        &["TASK_SHAPE"],
    )
}

#[test]
fn typed_ontology_resources_validate_before_store_calls() {
    let resolver = FakeScopeResolver::new(resolved_scope());
    let store = store_with_valid_resources();
    let service = OntologyService::new(store.clone(), resolver);

    let mut invalid_class = class_resource("bad code");
    invalid_class.label = " ".to_owned();
    assert_eq!(
        service.create_class(scope_command(), invalid_class),
        Err(OntologyError::InvalidOntologyCode)
    );

    let invalid_property = OntologyProperty {
        property_code: "OWNER".to_owned(),
        ontology_domain: OntologyDomain::Operations,
        label: "owner".to_owned(),
        description: None,
        domain_codes: Vec::new(),
        range_codes: vec!["USER_RANGE".to_owned()],
    };
    assert_eq!(
        service.create_property(scope_command(), invalid_property),
        Err(OntologyError::InvalidOntologyResource)
    );

    let invalid_domain = OntologyDomainResource {
        domain_code: "TASK_DOMAIN".to_owned(),
        ontology_domain: OntologyDomain::Operations,
        label: " ".to_owned(),
        description: None,
        class_code: "TASK".to_owned(),
    };
    assert_eq!(
        service.create_domain(scope_command(), invalid_domain),
        Err(OntologyError::InvalidOntologyResource)
    );

    let invalid_range = OntologyRangeResource {
        range_code: "USER_RANGE".to_owned(),
        ontology_domain: OntologyDomain::Operations,
        label: "user".to_owned(),
        description: None,
        target: OntologyRangeTarget::Class {
            class_code: "bad code".to_owned(),
        },
    };
    assert_eq!(
        service.create_range(scope_command(), invalid_range),
        Err(OntologyError::InvalidOntologyCode)
    );

    let invalid_constraint = OntologyConstraint {
        constraint_code: "OWNER_REQUIRED".to_owned(),
        ontology_domain: OntologyDomain::Operations,
        label: "required".to_owned(),
        description: None,
        required: false,
        min_count: Some(2),
        max_count: Some(1),
        allowed_range_codes: Vec::new(),
        pattern: None,
    };
    assert_eq!(
        service.create_constraint(scope_command(), invalid_constraint),
        Err(OntologyError::InconsistentOntologyResource)
    );

    let invalid_shape = OntologyShaclShape {
        shape_code: "TASK_SHAPE".to_owned(),
        ontology_domain: OntologyDomain::Operations,
        label: "task".to_owned(),
        description: None,
        target_class_code: "TASK".to_owned(),
        property_constraints: vec![OntologyShapeProperty {
            property_code: "OWNER".to_owned(),
            constraint_codes: Vec::new(),
        }],
    };
    assert_eq!(
        service.create_shape(scope_command(), invalid_shape),
        Err(OntologyError::InvalidOntologyResource)
    );

    assert_eq!(
        service.search(scope_command(), OntologySearchQuery::empty()),
        Err(OntologyError::EmptyOntologySearchQuery)
    );
    assert_eq!(store.calls(), Vec::new());
}

#[test]
fn duplicate_and_missing_store_outcomes_return_typed_ontology_errors() {
    let scope = resolved_scope();
    let resolver = FakeScopeResolver::new(scope);
    let store = store_with_valid_resources();
    let service = OntologyService::new(store.clone(), resolver);

    store.set_next_class_result(Err(OntologyError::DuplicateOntologyResource {
        resource_type: "class",
        resource_code: "TASK".to_owned(),
    }));
    assert_eq!(
        service.create_class(scope_command(), class_resource("TASK")),
        Err(OntologyError::DuplicateOntologyResource {
            resource_type: "class",
            resource_code: "TASK".to_owned(),
        })
    );

    assert_eq!(
        service.get_property(scope_command(), "OWNER_MISSING".to_owned()),
        Err(OntologyError::MissingOntologyResource {
            resource_type: "property",
            resource_code: "OWNER_MISSING".to_owned(),
        })
    );

    assert_eq!(
        service.get_domain(scope_command(), "TASK_DOMAIN_MISSING".to_owned()),
        Err(OntologyError::MissingOntologyResource {
            resource_type: "domain",
            resource_code: "TASK_DOMAIN_MISSING".to_owned(),
        })
    );

    assert_eq!(
        service.delete_range(scope_command(), "USER_RANGE_MISSING".to_owned()),
        Err(OntologyError::MissingOntologyResource {
            resource_type: "range",
            resource_code: "USER_RANGE_MISSING".to_owned(),
        })
    );

    store.set_next_constraint_result(Err(OntologyError::MissingOntologyResource {
        resource_type: "constraint",
        resource_code: "OWNER_REQUIRED_MISSING".to_owned(),
    }));
    assert_eq!(
        service.update_constraint(
            scope_command(),
            constraint_resource("OWNER_REQUIRED_MISSING"),
        ),
        Err(OntologyError::MissingOntologyResource {
            resource_type: "constraint",
            resource_code: "OWNER_REQUIRED_MISSING".to_owned(),
        })
    );

    assert_eq!(
        service.delete_shape(scope_command(), "TASK_SHAPE_MISSING".to_owned()),
        Err(OntologyError::MissingOntologyResource {
            resource_type: "shape",
            resource_code: "TASK_SHAPE_MISSING".to_owned(),
        })
    );
}

#[test]
fn missing_reference_dependencies_return_typed_errors_before_writes() {
    let scope = resolved_scope();

    let missing_parent_store = store_with_valid_resources();
    let missing_parent_service = OntologyService::new(
        missing_parent_store.clone(),
        FakeScopeResolver::new(scope.clone()),
    );
    let mut class = class_resource("TASK_CHILD");
    class.parent_class_codes = vec!["MISSING_PARENT".to_owned()];
    assert_eq!(
        missing_parent_service.create_class(scope_command(), class),
        Err(OntologyError::MissingOntologyResource {
            resource_type: "class",
            resource_code: "MISSING_PARENT".to_owned(),
        })
    );
    assert_eq!(
        missing_parent_store.calls(),
        vec![StoreCall::GetClass {
            scope: scope.clone(),
            class_code: "MISSING_PARENT".to_owned(),
        }]
    );

    let missing_domain_class_store = FakeStore::with_resources(
        &["ENTITY", "USER"],
        &["OWNER"],
        &["TASK_DOMAIN"],
        &["USER_RANGE"],
        &["OWNER_REQUIRED"],
        &["TASK_SHAPE"],
    );
    let missing_domain_class_service = OntologyService::new(
        missing_domain_class_store.clone(),
        FakeScopeResolver::new(scope.clone()),
    );
    assert_eq!(
        missing_domain_class_service.create_domain(
            scope_command(),
            OntologyDomainResource {
                class_code: "TASK".to_owned(),
                ..domain_resource("TASK_DOMAIN")
            },
        ),
        Err(OntologyError::MissingOntologyResource {
            resource_type: "class",
            resource_code: "TASK".to_owned(),
        })
    );
    assert_eq!(
        missing_domain_class_store.calls(),
        vec![StoreCall::GetClass {
            scope: scope.clone(),
            class_code: "TASK".to_owned(),
        }]
    );

    let missing_range_target_store = FakeStore::with_resources(
        &["ENTITY", "TASK"],
        &["OWNER"],
        &["TASK_DOMAIN"],
        &["USER_RANGE"],
        &["OWNER_REQUIRED"],
        &["TASK_SHAPE"],
    );
    let missing_range_target_service = OntologyService::new(
        missing_range_target_store.clone(),
        FakeScopeResolver::new(scope.clone()),
    );
    assert_eq!(
        missing_range_target_service.update_range(scope_command(), range_resource("USER_RANGE")),
        Err(OntologyError::MissingOntologyResource {
            resource_type: "class",
            resource_code: "USER".to_owned(),
        })
    );
    assert_eq!(
        missing_range_target_store.calls(),
        vec![StoreCall::GetClass {
            scope: scope.clone(),
            class_code: "USER".to_owned(),
        }]
    );

    let missing_property_range_store = FakeStore::with_resources(
        &["ENTITY", "TASK", "USER"],
        &["OWNER"],
        &["TASK_DOMAIN"],
        &[],
        &["OWNER_REQUIRED"],
        &["TASK_SHAPE"],
    );
    let missing_property_range_service = OntologyService::new(
        missing_property_range_store.clone(),
        FakeScopeResolver::new(scope.clone()),
    );
    assert_eq!(
        missing_property_range_service.update_property(scope_command(), property_resource("OWNER")),
        Err(OntologyError::MissingOntologyResource {
            resource_type: "range",
            resource_code: "USER_RANGE".to_owned(),
        })
    );
    assert_eq!(
        missing_property_range_store.calls(),
        vec![
            StoreCall::GetDomain {
                scope: scope.clone(),
                domain_code: "TASK_DOMAIN".to_owned(),
            },
            StoreCall::GetRange {
                scope: scope.clone(),
                range_code: "USER_RANGE".to_owned(),
            },
        ]
    );

    let missing_constraint_range_store = FakeStore::with_resources(
        &["ENTITY", "TASK", "USER"],
        &["OWNER"],
        &["TASK_DOMAIN"],
        &[],
        &["OWNER_REQUIRED"],
        &["TASK_SHAPE"],
    );
    let missing_constraint_range_service = OntologyService::new(
        missing_constraint_range_store.clone(),
        FakeScopeResolver::new(scope.clone()),
    );
    assert_eq!(
        missing_constraint_range_service
            .create_constraint(scope_command(), constraint_resource("OWNER_REQUIRED"),),
        Err(OntologyError::MissingOntologyResource {
            resource_type: "range",
            resource_code: "USER_RANGE".to_owned(),
        })
    );
    assert_eq!(
        missing_constraint_range_store.calls(),
        vec![StoreCall::GetRange {
            scope: scope.clone(),
            range_code: "USER_RANGE".to_owned(),
        }]
    );

    let missing_shape_property_store = FakeStore::with_resources(
        &["ENTITY", "TASK", "USER"],
        &[],
        &["TASK_DOMAIN"],
        &["USER_RANGE"],
        &["OWNER_REQUIRED"],
        &["TASK_SHAPE"],
    );
    let missing_shape_property_service = OntologyService::new(
        missing_shape_property_store.clone(),
        FakeScopeResolver::new(scope),
    );
    assert_eq!(
        missing_shape_property_service.create_shape(scope_command(), shape_resource("TASK_SHAPE")),
        Err(OntologyError::MissingOntologyResource {
            resource_type: "property",
            resource_code: "OWNER".to_owned(),
        })
    );
    assert_eq!(
        missing_shape_property_store.calls(),
        vec![
            StoreCall::GetClass {
                scope: resolved_scope(),
                class_code: "TASK".to_owned(),
            },
            StoreCall::GetProperty {
                scope: resolved_scope(),
                property_code: "OWNER".to_owned(),
            },
        ]
    );
}

#[test]
fn store_port_receives_resolved_scope_and_typed_ontology_requests() {
    let scope = resolved_scope();
    let resolver = FakeScopeResolver::new(scope.clone());
    let store = store_with_valid_resources();
    let service = OntologyService::new(store.clone(), resolver);
    let query = search_query();

    service
        .create_class(scope_command(), class_resource(" TASK "))
        .expect("create class should pass");
    service
        .create_domain(scope_command(), domain_resource(" TASK_DOMAIN "))
        .expect("create domain should pass");
    service
        .get_domain(scope_command(), "TASK_DOMAIN".to_owned())
        .expect("get domain should pass");
    service
        .update_domain(scope_command(), domain_resource("TASK_DOMAIN"))
        .expect("update domain should pass");
    service
        .delete_domain(scope_command(), "TASK_DOMAIN".to_owned())
        .expect("delete domain should pass");
    service
        .create_range(scope_command(), range_resource(" USER_RANGE "))
        .expect("create range should pass");
    service
        .get_range(scope_command(), "USER_RANGE".to_owned())
        .expect("get range should pass");
    service
        .update_range(scope_command(), range_resource("USER_RANGE"))
        .expect("update range should pass");
    service
        .delete_range(scope_command(), "USER_RANGE".to_owned())
        .expect("delete range should pass");
    service
        .update_property(scope_command(), property_resource("OWNER"))
        .expect("update property should pass");
    service
        .get_constraint(scope_command(), "OWNER_REQUIRED".to_owned())
        .expect("get constraint should pass");
    service
        .delete_shape(scope_command(), "TASK_SHAPE".to_owned())
        .expect("delete shape should pass");
    service
        .create_shape(scope_command(), shape_resource("TASK_SHAPE"))
        .expect("create shape should pass");
    service
        .search(scope_command(), query.clone())
        .expect("search should pass");

    assert_eq!(
        store.calls(),
        vec![
            StoreCall::GetClass {
                scope: scope.clone(),
                class_code: "ENTITY".to_owned(),
            },
            StoreCall::CreateClass {
                scope: scope.clone(),
                class: class_resource("TASK")
                    .validate()
                    .expect("class should validate"),
            },
            StoreCall::GetClass {
                scope: scope.clone(),
                class_code: "TASK".to_owned(),
            },
            StoreCall::CreateDomain {
                scope: scope.clone(),
                domain: domain_resource("TASK_DOMAIN")
                    .validate()
                    .expect("domain should validate"),
            },
            StoreCall::GetDomain {
                scope: scope.clone(),
                domain_code: "TASK_DOMAIN".to_owned(),
            },
            StoreCall::GetClass {
                scope: scope.clone(),
                class_code: "TASK".to_owned(),
            },
            StoreCall::UpdateDomain {
                scope: scope.clone(),
                domain: domain_resource("TASK_DOMAIN")
                    .validate()
                    .expect("domain should validate"),
            },
            StoreCall::DeleteDomain {
                scope: scope.clone(),
                domain_code: "TASK_DOMAIN".to_owned(),
            },
            StoreCall::GetClass {
                scope: scope.clone(),
                class_code: "USER".to_owned(),
            },
            StoreCall::CreateRange {
                scope: scope.clone(),
                range: range_resource("USER_RANGE")
                    .validate()
                    .expect("range should validate"),
            },
            StoreCall::GetRange {
                scope: scope.clone(),
                range_code: "USER_RANGE".to_owned(),
            },
            StoreCall::GetClass {
                scope: scope.clone(),
                class_code: "USER".to_owned(),
            },
            StoreCall::UpdateRange {
                scope: scope.clone(),
                range: range_resource("USER_RANGE")
                    .validate()
                    .expect("range should validate"),
            },
            StoreCall::DeleteRange {
                scope: scope.clone(),
                range_code: "USER_RANGE".to_owned(),
            },
            StoreCall::GetDomain {
                scope: scope.clone(),
                domain_code: "TASK_DOMAIN".to_owned(),
            },
            StoreCall::GetRange {
                scope: scope.clone(),
                range_code: "USER_RANGE".to_owned(),
            },
            StoreCall::UpdateProperty {
                scope: scope.clone(),
                property: property_resource("OWNER")
                    .validate()
                    .expect("property should validate"),
            },
            StoreCall::GetConstraint {
                scope: scope.clone(),
                constraint_code: "OWNER_REQUIRED".to_owned(),
            },
            StoreCall::DeleteShape {
                scope: scope.clone(),
                shape_code: "TASK_SHAPE".to_owned(),
            },
            StoreCall::GetClass {
                scope: scope.clone(),
                class_code: "TASK".to_owned(),
            },
            StoreCall::GetProperty {
                scope: scope.clone(),
                property_code: "OWNER".to_owned(),
            },
            StoreCall::GetConstraint {
                scope: scope.clone(),
                constraint_code: "OWNER_REQUIRED".to_owned(),
            },
            StoreCall::CreateShape {
                scope: scope.clone(),
                shape: shape_resource("TASK_SHAPE")
                    .validate()
                    .expect("shape should validate"),
            },
            StoreCall::Search {
                scope,
                query: query.validate().expect("query should validate"),
            },
        ]
    );
}
