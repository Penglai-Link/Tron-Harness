use std::sync::Arc;

use super::ontology_reference_validation::{
    ensure_code_exists, ensure_codes_exist, ensure_range_target_exists,
    ensure_shape_references_exist, missing_resource,
};
use crate::{
    application::graph::{ScopeResolver, WorkspaceDetailProvider},
    domain::graph::{
        GraphScopeCommand, OntologyClass, OntologyConstraint, OntologyDomainResource,
        OntologyError, OntologyProperty, OntologyRangeResource, OntologyResource,
        OntologyResourceType, OntologySearchQuery, OntologyShaclShape, ResolvedGraphScope,
        normalize_ontology_code,
    },
};

pub trait OntologyScopeResolver: Send + Sync {
    fn resolve_ontology_scope(
        &self,
        command: GraphScopeCommand,
    ) -> Result<ResolvedGraphScope, OntologyError>;
}

impl<W> OntologyScopeResolver for ScopeResolver<W>
where
    W: WorkspaceDetailProvider + Send + Sync,
{
    fn resolve_ontology_scope(
        &self,
        command: GraphScopeCommand,
    ) -> Result<ResolvedGraphScope, OntologyError> {
        self.resolve(command).map_err(OntologyError::from)
    }
}

pub trait OntologyStore: Send + Sync {
    fn create_class(
        &self,
        scope: &ResolvedGraphScope,
        class: OntologyClass,
    ) -> Result<OntologyClass, OntologyError>;

    fn get_class(
        &self,
        scope: &ResolvedGraphScope,
        class_code: &str,
    ) -> Result<Option<OntologyClass>, OntologyError>;

    fn update_class(
        &self,
        scope: &ResolvedGraphScope,
        class: OntologyClass,
    ) -> Result<OntologyClass, OntologyError>;

    fn delete_class(
        &self,
        scope: &ResolvedGraphScope,
        class_code: &str,
    ) -> Result<(), OntologyError>;

    fn create_property(
        &self,
        scope: &ResolvedGraphScope,
        property: OntologyProperty,
    ) -> Result<OntologyProperty, OntologyError>;

    fn get_property(
        &self,
        scope: &ResolvedGraphScope,
        property_code: &str,
    ) -> Result<Option<OntologyProperty>, OntologyError>;

    fn update_property(
        &self,
        scope: &ResolvedGraphScope,
        property: OntologyProperty,
    ) -> Result<OntologyProperty, OntologyError>;

    fn delete_property(
        &self,
        scope: &ResolvedGraphScope,
        property_code: &str,
    ) -> Result<(), OntologyError>;

    fn create_domain(
        &self,
        scope: &ResolvedGraphScope,
        domain: OntologyDomainResource,
    ) -> Result<OntologyDomainResource, OntologyError>;

    fn get_domain(
        &self,
        scope: &ResolvedGraphScope,
        domain_code: &str,
    ) -> Result<Option<OntologyDomainResource>, OntologyError>;

    fn update_domain(
        &self,
        scope: &ResolvedGraphScope,
        domain: OntologyDomainResource,
    ) -> Result<OntologyDomainResource, OntologyError>;

    fn delete_domain(
        &self,
        scope: &ResolvedGraphScope,
        domain_code: &str,
    ) -> Result<(), OntologyError>;

    fn create_range(
        &self,
        scope: &ResolvedGraphScope,
        range: OntologyRangeResource,
    ) -> Result<OntologyRangeResource, OntologyError>;

    fn get_range(
        &self,
        scope: &ResolvedGraphScope,
        range_code: &str,
    ) -> Result<Option<OntologyRangeResource>, OntologyError>;

    fn update_range(
        &self,
        scope: &ResolvedGraphScope,
        range: OntologyRangeResource,
    ) -> Result<OntologyRangeResource, OntologyError>;

    fn delete_range(
        &self,
        scope: &ResolvedGraphScope,
        range_code: &str,
    ) -> Result<(), OntologyError>;

    fn create_constraint(
        &self,
        scope: &ResolvedGraphScope,
        constraint: OntologyConstraint,
    ) -> Result<OntologyConstraint, OntologyError>;

    fn get_constraint(
        &self,
        scope: &ResolvedGraphScope,
        constraint_code: &str,
    ) -> Result<Option<OntologyConstraint>, OntologyError>;

    fn update_constraint(
        &self,
        scope: &ResolvedGraphScope,
        constraint: OntologyConstraint,
    ) -> Result<OntologyConstraint, OntologyError>;

    fn delete_constraint(
        &self,
        scope: &ResolvedGraphScope,
        constraint_code: &str,
    ) -> Result<(), OntologyError>;

    fn create_shape(
        &self,
        scope: &ResolvedGraphScope,
        shape: OntologyShaclShape,
    ) -> Result<OntologyShaclShape, OntologyError>;

    fn get_shape(
        &self,
        scope: &ResolvedGraphScope,
        shape_code: &str,
    ) -> Result<Option<OntologyShaclShape>, OntologyError>;

    fn update_shape(
        &self,
        scope: &ResolvedGraphScope,
        shape: OntologyShaclShape,
    ) -> Result<OntologyShaclShape, OntologyError>;

    fn delete_shape(
        &self,
        scope: &ResolvedGraphScope,
        shape_code: &str,
    ) -> Result<(), OntologyError>;

    fn search(
        &self,
        scope: &ResolvedGraphScope,
        query: OntologySearchQuery,
    ) -> Result<Vec<OntologyResource>, OntologyError>;
}

#[derive(Clone)]
pub struct OntologyService<R = ScopeResolver> {
    store: Arc<dyn OntologyStore>,
    scope_resolver: R,
}

impl<R> OntologyService<R>
where
    R: OntologyScopeResolver,
{
    pub fn new(store: impl OntologyStore + 'static, scope_resolver: R) -> Self {
        Self {
            store: Arc::new(store),
            scope_resolver,
        }
    }

    pub fn create_class(
        &self,
        scope: GraphScopeCommand,
        class: OntologyClass,
    ) -> Result<OntologyClass, OntologyError> {
        let class = class.validate()?;
        self.with_scope(scope, |store, scope| {
            ensure_codes_exist(
                OntologyResourceType::Class,
                &class.parent_class_codes,
                |code| Ok(store.get_class(scope, code)?.is_some()),
            )?;
            store.create_class(scope, class)
        })
    }

    pub fn get_class(
        &self,
        scope: GraphScopeCommand,
        class_code: String,
    ) -> Result<OntologyClass, OntologyError> {
        let class_code = normalize_ontology_code(&class_code)?;
        self.with_scope(scope, |store, scope| {
            store
                .get_class(scope, &class_code)?
                .ok_or_else(|| missing_resource(OntologyResourceType::Class, class_code))
        })
    }

    pub fn update_class(
        &self,
        scope: GraphScopeCommand,
        class: OntologyClass,
    ) -> Result<OntologyClass, OntologyError> {
        let class = class.validate()?;
        self.with_scope(scope, |store, scope| {
            ensure_codes_exist(
                OntologyResourceType::Class,
                &class.parent_class_codes,
                |code| Ok(store.get_class(scope, code)?.is_some()),
            )?;
            store.update_class(scope, class)
        })
    }

    pub fn delete_class(
        &self,
        scope: GraphScopeCommand,
        class_code: String,
    ) -> Result<(), OntologyError> {
        let class_code = normalize_ontology_code(&class_code)?;
        self.with_scope(scope, |store, scope| store.delete_class(scope, &class_code))
    }

    pub fn create_property(
        &self,
        scope: GraphScopeCommand,
        property: OntologyProperty,
    ) -> Result<OntologyProperty, OntologyError> {
        let property = property.validate()?;
        self.with_scope(scope, |store, scope| {
            ensure_codes_exist(
                OntologyResourceType::Domain,
                &property.domain_codes,
                |code| Ok(store.get_domain(scope, code)?.is_some()),
            )?;
            ensure_codes_exist(OntologyResourceType::Range, &property.range_codes, |code| {
                Ok(store.get_range(scope, code)?.is_some())
            })?;
            store.create_property(scope, property)
        })
    }

    pub fn get_property(
        &self,
        scope: GraphScopeCommand,
        property_code: String,
    ) -> Result<OntologyProperty, OntologyError> {
        let property_code = normalize_ontology_code(&property_code)?;
        self.with_scope(scope, |store, scope| {
            store
                .get_property(scope, &property_code)?
                .ok_or_else(|| missing_resource(OntologyResourceType::Property, property_code))
        })
    }

    pub fn update_property(
        &self,
        scope: GraphScopeCommand,
        property: OntologyProperty,
    ) -> Result<OntologyProperty, OntologyError> {
        let property = property.validate()?;
        self.with_scope(scope, |store, scope| {
            ensure_codes_exist(
                OntologyResourceType::Domain,
                &property.domain_codes,
                |code| Ok(store.get_domain(scope, code)?.is_some()),
            )?;
            ensure_codes_exist(OntologyResourceType::Range, &property.range_codes, |code| {
                Ok(store.get_range(scope, code)?.is_some())
            })?;
            store.update_property(scope, property)
        })
    }

    pub fn delete_property(
        &self,
        scope: GraphScopeCommand,
        property_code: String,
    ) -> Result<(), OntologyError> {
        let property_code = normalize_ontology_code(&property_code)?;
        self.with_scope(scope, |store, scope| {
            store.delete_property(scope, &property_code)
        })
    }

    pub fn create_domain(
        &self,
        scope: GraphScopeCommand,
        domain: OntologyDomainResource,
    ) -> Result<OntologyDomainResource, OntologyError> {
        let domain = domain.validate()?;
        self.with_scope(scope, |store, scope| {
            ensure_code_exists(OntologyResourceType::Class, &domain.class_code, |code| {
                Ok(store.get_class(scope, code)?.is_some())
            })?;
            store.create_domain(scope, domain)
        })
    }

    pub fn get_domain(
        &self,
        scope: GraphScopeCommand,
        domain_code: String,
    ) -> Result<OntologyDomainResource, OntologyError> {
        let domain_code = normalize_ontology_code(&domain_code)?;
        self.with_scope(scope, |store, scope| {
            store
                .get_domain(scope, &domain_code)?
                .ok_or_else(|| missing_resource(OntologyResourceType::Domain, domain_code))
        })
    }

    pub fn update_domain(
        &self,
        scope: GraphScopeCommand,
        domain: OntologyDomainResource,
    ) -> Result<OntologyDomainResource, OntologyError> {
        let domain = domain.validate()?;
        self.with_scope(scope, |store, scope| {
            ensure_code_exists(OntologyResourceType::Class, &domain.class_code, |code| {
                Ok(store.get_class(scope, code)?.is_some())
            })?;
            store.update_domain(scope, domain)
        })
    }

    pub fn delete_domain(
        &self,
        scope: GraphScopeCommand,
        domain_code: String,
    ) -> Result<(), OntologyError> {
        let domain_code = normalize_ontology_code(&domain_code)?;
        self.with_scope(scope, |store, scope| {
            store.delete_domain(scope, &domain_code)
        })
    }

    pub fn create_range(
        &self,
        scope: GraphScopeCommand,
        range: OntologyRangeResource,
    ) -> Result<OntologyRangeResource, OntologyError> {
        let range = range.validate()?;
        self.with_scope(scope, |store, scope| {
            ensure_range_target_exists(store, scope, &range.target)?;
            store.create_range(scope, range)
        })
    }

    pub fn get_range(
        &self,
        scope: GraphScopeCommand,
        range_code: String,
    ) -> Result<OntologyRangeResource, OntologyError> {
        let range_code = normalize_ontology_code(&range_code)?;
        self.with_scope(scope, |store, scope| {
            store
                .get_range(scope, &range_code)?
                .ok_or_else(|| missing_resource(OntologyResourceType::Range, range_code))
        })
    }

    pub fn update_range(
        &self,
        scope: GraphScopeCommand,
        range: OntologyRangeResource,
    ) -> Result<OntologyRangeResource, OntologyError> {
        let range = range.validate()?;
        self.with_scope(scope, |store, scope| {
            ensure_range_target_exists(store, scope, &range.target)?;
            store.update_range(scope, range)
        })
    }

    pub fn delete_range(
        &self,
        scope: GraphScopeCommand,
        range_code: String,
    ) -> Result<(), OntologyError> {
        let range_code = normalize_ontology_code(&range_code)?;
        self.with_scope(scope, |store, scope| store.delete_range(scope, &range_code))
    }

    pub fn create_constraint(
        &self,
        scope: GraphScopeCommand,
        constraint: OntologyConstraint,
    ) -> Result<OntologyConstraint, OntologyError> {
        let constraint = constraint.validate()?;
        self.with_scope(scope, |store, scope| {
            ensure_codes_exist(
                OntologyResourceType::Range,
                &constraint.allowed_range_codes,
                |code| Ok(store.get_range(scope, code)?.is_some()),
            )?;
            store.create_constraint(scope, constraint)
        })
    }

    pub fn get_constraint(
        &self,
        scope: GraphScopeCommand,
        constraint_code: String,
    ) -> Result<OntologyConstraint, OntologyError> {
        let constraint_code = normalize_ontology_code(&constraint_code)?;
        self.with_scope(scope, |store, scope| {
            store
                .get_constraint(scope, &constraint_code)?
                .ok_or_else(|| missing_resource(OntologyResourceType::Constraint, constraint_code))
        })
    }

    pub fn update_constraint(
        &self,
        scope: GraphScopeCommand,
        constraint: OntologyConstraint,
    ) -> Result<OntologyConstraint, OntologyError> {
        let constraint = constraint.validate()?;
        self.with_scope(scope, |store, scope| {
            ensure_codes_exist(
                OntologyResourceType::Range,
                &constraint.allowed_range_codes,
                |code| Ok(store.get_range(scope, code)?.is_some()),
            )?;
            store.update_constraint(scope, constraint)
        })
    }

    pub fn delete_constraint(
        &self,
        scope: GraphScopeCommand,
        constraint_code: String,
    ) -> Result<(), OntologyError> {
        let constraint_code = normalize_ontology_code(&constraint_code)?;
        self.with_scope(scope, |store, scope| {
            store.delete_constraint(scope, &constraint_code)
        })
    }

    pub fn create_shape(
        &self,
        scope: GraphScopeCommand,
        shape: OntologyShaclShape,
    ) -> Result<OntologyShaclShape, OntologyError> {
        let shape = shape.validate()?;
        self.with_scope(scope, |store, scope| {
            ensure_shape_references_exist(store, scope, &shape)?;
            store.create_shape(scope, shape)
        })
    }

    pub fn get_shape(
        &self,
        scope: GraphScopeCommand,
        shape_code: String,
    ) -> Result<OntologyShaclShape, OntologyError> {
        let shape_code = normalize_ontology_code(&shape_code)?;
        self.with_scope(scope, |store, scope| {
            store
                .get_shape(scope, &shape_code)?
                .ok_or_else(|| missing_resource(OntologyResourceType::Shape, shape_code))
        })
    }

    pub fn update_shape(
        &self,
        scope: GraphScopeCommand,
        shape: OntologyShaclShape,
    ) -> Result<OntologyShaclShape, OntologyError> {
        let shape = shape.validate()?;
        self.with_scope(scope, |store, scope| {
            ensure_shape_references_exist(store, scope, &shape)?;
            store.update_shape(scope, shape)
        })
    }

    pub fn delete_shape(
        &self,
        scope: GraphScopeCommand,
        shape_code: String,
    ) -> Result<(), OntologyError> {
        let shape_code = normalize_ontology_code(&shape_code)?;
        self.with_scope(scope, |store, scope| store.delete_shape(scope, &shape_code))
    }

    pub fn search(
        &self,
        scope: GraphScopeCommand,
        query: OntologySearchQuery,
    ) -> Result<Vec<OntologyResource>, OntologyError> {
        let query = query.validate()?;
        self.with_scope(scope, |store, scope| store.search(scope, query))
    }

    fn with_scope<T>(
        &self,
        scope: GraphScopeCommand,
        operation: impl FnOnce(&dyn OntologyStore, &ResolvedGraphScope) -> Result<T, OntologyError>,
    ) -> Result<T, OntologyError> {
        let scope = self.scope_resolver.resolve_ontology_scope(scope)?;
        operation(self.store.as_ref(), &scope)
    }
}
