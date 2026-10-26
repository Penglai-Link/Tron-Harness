use super::{
    grafeo_core::GrafeoStoreCore,
    grafeo_ontology_support::{
        StoredOntologyRecord, base_scope_params, code_match, exact_scope_match, label_match,
        load_resources, ontology_domain_match, resource_params, resource_type_match, storage_error,
        target_class_match, visible_scope_match,
    },
};
use crate::{
    application::graph::OntologyStore,
    domain::graph::{
        OntologyClass, OntologyConstraint, OntologyDomainResource, OntologyError, OntologyProperty,
        OntologyRangeResource, OntologyResource, OntologyResourceType, OntologySearchQuery,
        OntologyShaclShape, ResolvedGraphScope,
    },
};

#[derive(Clone)]
pub struct GrafeoOntologyStore {
    core: GrafeoStoreCore,
}

impl GrafeoOntologyStore {
    pub fn new(core: GrafeoStoreCore) -> Self {
        Self { core }
    }

    fn get_exact_resource(
        &self,
        session: &grafeo::Session,
        scope: &ResolvedGraphScope,
        resource_type: OntologyResourceType,
        resource_code: &str,
    ) -> Result<Option<StoredOntologyRecord>, OntologyError> {
        Ok(load_resources(session)?.into_iter().find(|record| {
            record.resource_type == resource_type
                && record.resource_code == resource_code
                && exact_scope_match(record, scope)
        }))
    }

    fn get_visible_resource(
        &self,
        session: &grafeo::Session,
        scope: &ResolvedGraphScope,
        resource_type: OntologyResourceType,
        resource_code: &str,
    ) -> Result<Option<OntologyResource>, OntologyError> {
        Ok(load_resources(session)?
            .into_iter()
            .filter(|record| visible_scope_match(record, scope))
            .find(|record| {
                record.resource_type == resource_type && record.resource_code == resource_code
            })
            .map(|record| record.resource))
    }

    fn create_resource(
        &self,
        scope: &ResolvedGraphScope,
        record: StoredOntologyRecord,
    ) -> Result<OntologyResource, OntologyError> {
        let session = self.core.session_for_scope(scope).map_err(storage_error)?;
        if self
            .get_exact_resource(&session, scope, record.resource_type, &record.resource_code)?
            .is_some()
        {
            return Err(OntologyError::DuplicateOntologyResource {
                resource_type: record.resource_type.as_str(),
                resource_code: record.resource_code,
            });
        }

        session
            .execute_with_params(
                "INSERT (:TronOntologyResource { \
                    resourceType: $resourceType, resourceCode: $resourceCode, ontologyDomainKey: $ontologyDomainKey, \
                    resourceLabel: $resourceLabel, targetClassCode: $targetClassCode, resourceJson: $resourceJson, \
                    namespace: $namespace, workspaceCode: $workspaceCode, appCode: $appCode, userCode: $userCode \
                })",
                resource_params(scope, &record),
            )
            .map_err(storage_error)?;

        self.get_exact_resource(&session, scope, record.resource_type, &record.resource_code)?
            .map(|stored| stored.resource)
            .ok_or(OntologyError::StorageFailure)
    }

    fn update_resource(
        &self,
        scope: &ResolvedGraphScope,
        record: StoredOntologyRecord,
    ) -> Result<OntologyResource, OntologyError> {
        let session = self.core.session_for_scope(scope).map_err(storage_error)?;
        if self
            .get_exact_resource(&session, scope, record.resource_type, &record.resource_code)?
            .is_none()
        {
            return Err(OntologyError::MissingOntologyResource {
                resource_type: record.resource_type.as_str(),
                resource_code: record.resource_code,
            });
        }

        session
            .execute_with_params(
                "MATCH (n:TronOntologyResource) \
                 WHERE n.resourceType = $resourceType AND n.resourceCode = $resourceCode \
                   AND n.namespace = $namespace AND n.workspaceCode = $workspaceCode \
                   AND n.appCode = $appCode AND n.userCode = $userCode \
                 SET n.ontologyDomainKey = $ontologyDomainKey, n.resourceLabel = $resourceLabel, \
                     n.targetClassCode = $targetClassCode, n.resourceJson = $resourceJson",
                resource_params(scope, &record),
            )
            .map_err(storage_error)?;

        self.get_exact_resource(&session, scope, record.resource_type, &record.resource_code)?
            .map(|stored| stored.resource)
            .ok_or(OntologyError::StorageFailure)
    }

    fn delete_resource(
        &self,
        scope: &ResolvedGraphScope,
        resource_type: OntologyResourceType,
        resource_code: &str,
    ) -> Result<(), OntologyError> {
        let session = self.core.session_for_scope(scope).map_err(storage_error)?;
        if self
            .get_exact_resource(&session, scope, resource_type, resource_code)?
            .is_none()
        {
            return Err(OntologyError::MissingOntologyResource {
                resource_type: resource_type.as_str(),
                resource_code: resource_code.to_owned(),
            });
        }

        let mut params = base_scope_params(scope);
        params.insert(
            "resourceType".into(),
            grafeo::Value::from(resource_type.as_str()),
        );
        params.insert(
            "resourceCode".into(),
            grafeo::Value::from(resource_code.to_owned()),
        );
        session
            .execute_with_params(
                "MATCH (n:TronOntologyResource) \
                 WHERE n.resourceType = $resourceType AND n.resourceCode = $resourceCode \
                   AND n.namespace = $namespace AND n.workspaceCode = $workspaceCode \
                   AND n.appCode = $appCode AND n.userCode = $userCode \
                 DELETE n",
                params,
            )
            .map_err(storage_error)?;
        Ok(())
    }
}

impl OntologyStore for GrafeoOntologyStore {
    fn resources_present(
        &self,
        scope: &ResolvedGraphScope,
        references: &[(OntologyResourceType, String)],
    ) -> Result<Vec<bool>, OntologyError> {
        if references.is_empty() {
            return Ok(Vec::new());
        }
        let session = self.core.session_for_scope(scope).map_err(storage_error)?;
        let visible = load_resources(&session)?
            .into_iter()
            .filter(|record| visible_scope_match(record, scope))
            .map(|record| (record.resource_type.as_str(), record.resource_code))
            .collect::<std::collections::HashSet<_>>();
        Ok(references
            .iter()
            .map(|(kind, code)| visible.contains(&(kind.as_str(), code.clone())))
            .collect())
    }
    fn create_class(
        &self,
        scope: &ResolvedGraphScope,
        class: OntologyClass,
    ) -> Result<OntologyClass, OntologyError> {
        match self.create_resource(
            scope,
            StoredOntologyRecord::from_resource(OntologyResource::Class(class))?,
        )? {
            OntologyResource::Class(class) => Ok(class),
            _ => Err(OntologyError::StorageFailure),
        }
    }

    fn get_class(
        &self,
        scope: &ResolvedGraphScope,
        class_code: &str,
    ) -> Result<Option<OntologyClass>, OntologyError> {
        let session = self.core.session_for_scope(scope).map_err(storage_error)?;
        match self.get_visible_resource(&session, scope, OntologyResourceType::Class, class_code)? {
            Some(OntologyResource::Class(class)) => Ok(Some(class)),
            Some(_) => Err(OntologyError::StorageFailure),
            None => Ok(None),
        }
    }

    fn update_class(
        &self,
        scope: &ResolvedGraphScope,
        class: OntologyClass,
    ) -> Result<OntologyClass, OntologyError> {
        match self.update_resource(
            scope,
            StoredOntologyRecord::from_resource(OntologyResource::Class(class))?,
        )? {
            OntologyResource::Class(class) => Ok(class),
            _ => Err(OntologyError::StorageFailure),
        }
    }

    fn delete_class(
        &self,
        scope: &ResolvedGraphScope,
        class_code: &str,
    ) -> Result<(), OntologyError> {
        self.delete_resource(scope, OntologyResourceType::Class, class_code)
    }

    fn create_property(
        &self,
        scope: &ResolvedGraphScope,
        property: OntologyProperty,
    ) -> Result<OntologyProperty, OntologyError> {
        match self.create_resource(
            scope,
            StoredOntologyRecord::from_resource(OntologyResource::Property(property))?,
        )? {
            OntologyResource::Property(property) => Ok(property),
            _ => Err(OntologyError::StorageFailure),
        }
    }

    fn get_property(
        &self,
        scope: &ResolvedGraphScope,
        property_code: &str,
    ) -> Result<Option<OntologyProperty>, OntologyError> {
        let session = self.core.session_for_scope(scope).map_err(storage_error)?;
        match self.get_visible_resource(
            &session,
            scope,
            OntologyResourceType::Property,
            property_code,
        )? {
            Some(OntologyResource::Property(property)) => Ok(Some(property)),
            Some(_) => Err(OntologyError::StorageFailure),
            None => Ok(None),
        }
    }

    fn update_property(
        &self,
        scope: &ResolvedGraphScope,
        property: OntologyProperty,
    ) -> Result<OntologyProperty, OntologyError> {
        match self.update_resource(
            scope,
            StoredOntologyRecord::from_resource(OntologyResource::Property(property))?,
        )? {
            OntologyResource::Property(property) => Ok(property),
            _ => Err(OntologyError::StorageFailure),
        }
    }

    fn delete_property(
        &self,
        scope: &ResolvedGraphScope,
        property_code: &str,
    ) -> Result<(), OntologyError> {
        self.delete_resource(scope, OntologyResourceType::Property, property_code)
    }

    fn create_domain(
        &self,
        scope: &ResolvedGraphScope,
        domain: OntologyDomainResource,
    ) -> Result<OntologyDomainResource, OntologyError> {
        match self.create_resource(
            scope,
            StoredOntologyRecord::from_resource(OntologyResource::Domain(domain))?,
        )? {
            OntologyResource::Domain(domain) => Ok(domain),
            _ => Err(OntologyError::StorageFailure),
        }
    }

    fn get_domain(
        &self,
        scope: &ResolvedGraphScope,
        domain_code: &str,
    ) -> Result<Option<OntologyDomainResource>, OntologyError> {
        let session = self.core.session_for_scope(scope).map_err(storage_error)?;
        match self.get_visible_resource(
            &session,
            scope,
            OntologyResourceType::Domain,
            domain_code,
        )? {
            Some(OntologyResource::Domain(domain)) => Ok(Some(domain)),
            Some(_) => Err(OntologyError::StorageFailure),
            None => Ok(None),
        }
    }

    fn update_domain(
        &self,
        scope: &ResolvedGraphScope,
        domain: OntologyDomainResource,
    ) -> Result<OntologyDomainResource, OntologyError> {
        match self.update_resource(
            scope,
            StoredOntologyRecord::from_resource(OntologyResource::Domain(domain))?,
        )? {
            OntologyResource::Domain(domain) => Ok(domain),
            _ => Err(OntologyError::StorageFailure),
        }
    }

    fn delete_domain(
        &self,
        scope: &ResolvedGraphScope,
        domain_code: &str,
    ) -> Result<(), OntologyError> {
        self.delete_resource(scope, OntologyResourceType::Domain, domain_code)
    }

    fn create_range(
        &self,
        scope: &ResolvedGraphScope,
        range: OntologyRangeResource,
    ) -> Result<OntologyRangeResource, OntologyError> {
        match self.create_resource(
            scope,
            StoredOntologyRecord::from_resource(OntologyResource::Range(range))?,
        )? {
            OntologyResource::Range(range) => Ok(range),
            _ => Err(OntologyError::StorageFailure),
        }
    }

    fn get_range(
        &self,
        scope: &ResolvedGraphScope,
        range_code: &str,
    ) -> Result<Option<OntologyRangeResource>, OntologyError> {
        let session = self.core.session_for_scope(scope).map_err(storage_error)?;
        match self.get_visible_resource(&session, scope, OntologyResourceType::Range, range_code)? {
            Some(OntologyResource::Range(range)) => Ok(Some(range)),
            Some(_) => Err(OntologyError::StorageFailure),
            None => Ok(None),
        }
    }

    fn update_range(
        &self,
        scope: &ResolvedGraphScope,
        range: OntologyRangeResource,
    ) -> Result<OntologyRangeResource, OntologyError> {
        match self.update_resource(
            scope,
            StoredOntologyRecord::from_resource(OntologyResource::Range(range))?,
        )? {
            OntologyResource::Range(range) => Ok(range),
            _ => Err(OntologyError::StorageFailure),
        }
    }

    fn delete_range(
        &self,
        scope: &ResolvedGraphScope,
        range_code: &str,
    ) -> Result<(), OntologyError> {
        self.delete_resource(scope, OntologyResourceType::Range, range_code)
    }

    fn create_constraint(
        &self,
        scope: &ResolvedGraphScope,
        constraint: OntologyConstraint,
    ) -> Result<OntologyConstraint, OntologyError> {
        match self.create_resource(
            scope,
            StoredOntologyRecord::from_resource(OntologyResource::Constraint(constraint))?,
        )? {
            OntologyResource::Constraint(constraint) => Ok(constraint),
            _ => Err(OntologyError::StorageFailure),
        }
    }

    fn get_constraint(
        &self,
        scope: &ResolvedGraphScope,
        constraint_code: &str,
    ) -> Result<Option<OntologyConstraint>, OntologyError> {
        let session = self.core.session_for_scope(scope).map_err(storage_error)?;
        match self.get_visible_resource(
            &session,
            scope,
            OntologyResourceType::Constraint,
            constraint_code,
        )? {
            Some(OntologyResource::Constraint(constraint)) => Ok(Some(constraint)),
            Some(_) => Err(OntologyError::StorageFailure),
            None => Ok(None),
        }
    }

    fn update_constraint(
        &self,
        scope: &ResolvedGraphScope,
        constraint: OntologyConstraint,
    ) -> Result<OntologyConstraint, OntologyError> {
        match self.update_resource(
            scope,
            StoredOntologyRecord::from_resource(OntologyResource::Constraint(constraint))?,
        )? {
            OntologyResource::Constraint(constraint) => Ok(constraint),
            _ => Err(OntologyError::StorageFailure),
        }
    }

    fn delete_constraint(
        &self,
        scope: &ResolvedGraphScope,
        constraint_code: &str,
    ) -> Result<(), OntologyError> {
        self.delete_resource(scope, OntologyResourceType::Constraint, constraint_code)
    }

    fn create_shape(
        &self,
        scope: &ResolvedGraphScope,
        shape: OntologyShaclShape,
    ) -> Result<OntologyShaclShape, OntologyError> {
        match self.create_resource(
            scope,
            StoredOntologyRecord::from_resource(OntologyResource::Shape(shape))?,
        )? {
            OntologyResource::Shape(shape) => Ok(shape),
            _ => Err(OntologyError::StorageFailure),
        }
    }

    fn get_shape(
        &self,
        scope: &ResolvedGraphScope,
        shape_code: &str,
    ) -> Result<Option<OntologyShaclShape>, OntologyError> {
        let session = self.core.session_for_scope(scope).map_err(storage_error)?;
        match self.get_visible_resource(&session, scope, OntologyResourceType::Shape, shape_code)? {
            Some(OntologyResource::Shape(shape)) => Ok(Some(shape)),
            Some(_) => Err(OntologyError::StorageFailure),
            None => Ok(None),
        }
    }

    fn update_shape(
        &self,
        scope: &ResolvedGraphScope,
        shape: OntologyShaclShape,
    ) -> Result<OntologyShaclShape, OntologyError> {
        match self.update_resource(
            scope,
            StoredOntologyRecord::from_resource(OntologyResource::Shape(shape))?,
        )? {
            OntologyResource::Shape(shape) => Ok(shape),
            _ => Err(OntologyError::StorageFailure),
        }
    }

    fn delete_shape(
        &self,
        scope: &ResolvedGraphScope,
        shape_code: &str,
    ) -> Result<(), OntologyError> {
        self.delete_resource(scope, OntologyResourceType::Shape, shape_code)
    }

    fn search(
        &self,
        scope: &ResolvedGraphScope,
        query: OntologySearchQuery,
    ) -> Result<Vec<OntologyResource>, OntologyError> {
        let session = self.core.session_for_scope(scope).map_err(storage_error)?;
        let mut resources = Vec::new();
        for record in load_resources(&session)?
            .into_iter()
            .filter(|record| visible_scope_match(record, scope))
        {
            if !resource_type_match(&record, &query)
                || !ontology_domain_match(&record, &query)
                || !code_match(&record, &query)
                || !label_match(&record, &query)
                || !target_class_match(&record, &query)
            {
                continue;
            }

            resources.push(record.resource.clone());
            if resources.len() >= query.limit.value() {
                break;
            }
        }
        Ok(resources)
    }
}
