use std::collections::HashMap;

use grafeo::{Session, Value};

use crate::domain::graph::{
    OntologyDomain, OntologyError, OntologyRangeTarget, OntologyResource, OntologyResourceType,
    OntologySearchQuery, ResolvedGraphScope,
};

const RESOURCE_QUERY: &str = "MATCH (n:TronOntologyResource) RETURN \
    n.resourceType, n.resourceCode, n.ontologyDomainKey, n.resourceLabel, \
    n.targetClassCode, n.resourceJson, n.namespace, n.workspaceCode, n.appCode, n.userCode";

#[derive(Clone)]
pub(super) struct StoredOntologyRecord {
    pub resource: OntologyResource,
    pub resource_type: OntologyResourceType,
    pub resource_code: String,
    pub ontology_domain_key: String,
    pub resource_label: String,
    pub target_class_code: Option<String>,
    pub namespace: String,
    pub workspace_code: String,
    pub app_code: String,
    pub user_code: String,
    pub resource_json: String,
}

pub(super) trait ScopedRecord {
    fn namespace(&self) -> &str;
    fn workspace_code(&self) -> &str;
    fn app_code(&self) -> &str;
    fn user_code(&self) -> &str;
}

impl ScopedRecord for StoredOntologyRecord {
    fn namespace(&self) -> &str {
        &self.namespace
    }

    fn workspace_code(&self) -> &str {
        &self.workspace_code
    }

    fn app_code(&self) -> &str {
        &self.app_code
    }

    fn user_code(&self) -> &str {
        &self.user_code
    }
}

impl StoredOntologyRecord {
    pub(super) fn from_resource(resource: OntologyResource) -> Result<Self, OntologyError> {
        let (resource_type, resource_code, ontology_domain, resource_label, target_class_code) =
            match &resource {
                OntologyResource::Class(class) => (
                    OntologyResourceType::Class,
                    class.class_code.clone(),
                    class.ontology_domain.clone(),
                    class.label.clone(),
                    None,
                ),
                OntologyResource::Property(property) => (
                    OntologyResourceType::Property,
                    property.property_code.clone(),
                    property.ontology_domain.clone(),
                    property.label.clone(),
                    None,
                ),
                OntologyResource::Domain(domain) => (
                    OntologyResourceType::Domain,
                    domain.domain_code.clone(),
                    domain.ontology_domain.clone(),
                    domain.label.clone(),
                    Some(domain.class_code.clone()),
                ),
                OntologyResource::Range(range) => (
                    OntologyResourceType::Range,
                    range.range_code.clone(),
                    range.ontology_domain.clone(),
                    range.label.clone(),
                    match &range.target {
                        OntologyRangeTarget::Class { class_code } => Some(class_code.clone()),
                        OntologyRangeTarget::DataType(_) => None,
                    },
                ),
                OntologyResource::Constraint(constraint) => (
                    OntologyResourceType::Constraint,
                    constraint.constraint_code.clone(),
                    constraint.ontology_domain.clone(),
                    constraint.label.clone(),
                    None,
                ),
                OntologyResource::Shape(shape) => (
                    OntologyResourceType::Shape,
                    shape.shape_code.clone(),
                    shape.ontology_domain.clone(),
                    shape.label.clone(),
                    Some(shape.target_class_code.clone()),
                ),
            };

        Ok(Self {
            resource_json: serde_json::to_string(&resource)
                .map_err(|_| OntologyError::StorageFailure)?,
            resource,
            resource_type,
            resource_code,
            ontology_domain_key: ontology_domain_key(&ontology_domain),
            resource_label,
            target_class_code,
            namespace: String::new(),
            workspace_code: String::new(),
            app_code: String::new(),
            user_code: String::new(),
        })
    }
}

pub(super) fn load_resources(
    session: &Session,
) -> Result<Vec<StoredOntologyRecord>, OntologyError> {
    let result = session.execute(RESOURCE_QUERY).map_err(storage_error)?;
    result
        .rows()
        .iter()
        .map(|row| parse_resource_row(row))
        .collect::<Result<Vec<_>, _>>()
}

pub(super) fn resource_params(
    scope: &ResolvedGraphScope,
    record: &StoredOntologyRecord,
) -> HashMap<String, Value> {
    let mut params = base_scope_params(scope);
    params.insert(
        "resourceType".into(),
        Value::from(record.resource_type.as_str()),
    );
    params.insert(
        "resourceCode".into(),
        Value::from(record.resource_code.clone()),
    );
    params.insert(
        "ontologyDomainKey".into(),
        Value::from(record.ontology_domain_key.clone()),
    );
    params.insert(
        "resourceLabel".into(),
        Value::from(record.resource_label.clone()),
    );
    params.insert(
        "targetClassCode".into(),
        record
            .target_class_code
            .clone()
            .map(Value::from)
            .unwrap_or(Value::Null),
    );
    params.insert(
        "resourceJson".into(),
        Value::from(record.resource_json.clone()),
    );
    params
}

pub(super) fn base_scope_params(scope: &ResolvedGraphScope) -> HashMap<String, Value> {
    HashMap::from([
        (
            "namespace".into(),
            Value::from(scope.storage_dimensions.namespace.clone()),
        ),
        (
            "workspaceCode".into(),
            Value::from(scope.storage_dimensions.workspace_code.clone()),
        ),
        (
            "appCode".into(),
            Value::from(scope.storage_dimensions.app_code.clone()),
        ),
        (
            "userCode".into(),
            Value::from(scope.storage_dimensions.user_code.clone()),
        ),
    ])
}

pub(super) fn storage_error(_error: grafeo::Error) -> OntologyError {
    OntologyError::StorageFailure
}

pub(super) fn exact_scope_match(record: &impl ScopedRecord, scope: &ResolvedGraphScope) -> bool {
    record.namespace() == scope.storage_dimensions.namespace
        && record.workspace_code() == scope.storage_dimensions.workspace_code
        && record.app_code() == scope.storage_dimensions.app_code
        && record.user_code() == scope.storage_dimensions.user_code
}

pub(super) fn visible_scope_match(record: &impl ScopedRecord, scope: &ResolvedGraphScope) -> bool {
    record.namespace() == scope.storage_dimensions.namespace
        && record.app_code() == scope.storage_dimensions.app_code
        && record.user_code() == scope.storage_dimensions.user_code
        && scope
            .visible_workspace_codes
            .contains(&record.workspace_code().to_owned())
}

pub(super) fn resource_type_match(
    record: &StoredOntologyRecord,
    query: &OntologySearchQuery,
) -> bool {
    query.resource_types.is_empty() || query.resource_types.contains(&record.resource_type)
}

pub(super) fn ontology_domain_match(
    record: &StoredOntologyRecord,
    query: &OntologySearchQuery,
) -> bool {
    query.ontology_domains.is_empty()
        || query
            .ontology_domains
            .iter()
            .any(|domain| ontology_domain_key(domain) == record.ontology_domain_key)
}

pub(super) fn code_match(record: &StoredOntologyRecord, query: &OntologySearchQuery) -> bool {
    query.codes.is_empty() || query.codes.iter().any(|code| code == &record.resource_code)
}

pub(super) fn label_match(record: &StoredOntologyRecord, query: &OntologySearchQuery) -> bool {
    let Some(label) = &query.label else {
        return true;
    };
    record
        .resource_label
        .to_lowercase()
        .contains(&label.to_lowercase())
}

pub(super) fn target_class_match(
    record: &StoredOntologyRecord,
    query: &OntologySearchQuery,
) -> bool {
    let Some(target_class_code) = &query.target_class_code else {
        return true;
    };
    record
        .target_class_code
        .as_ref()
        .map(|value| value == target_class_code)
        .unwrap_or(false)
}

fn parse_resource_row(row: &[Value]) -> Result<StoredOntologyRecord, OntologyError> {
    let resource_json = required_string(&row[5])?;
    let resource = serde_json::from_str::<OntologyResource>(&resource_json)
        .map_err(|_| OntologyError::StorageFailure)?;
    Ok(StoredOntologyRecord {
        resource,
        resource_type: parse_resource_type(&row[0])?,
        resource_code: required_string(&row[1])?,
        ontology_domain_key: required_string(&row[2])?,
        resource_label: required_string(&row[3])?,
        target_class_code: optional_string(&row[4]),
        resource_json,
        namespace: required_string(&row[6])?,
        workspace_code: required_string(&row[7])?,
        app_code: required_string(&row[8])?,
        user_code: required_string(&row[9])?,
    })
}

fn required_string(value: &Value) -> Result<String, OntologyError> {
    value
        .as_str()
        .map(|value| value.to_owned())
        .ok_or(OntologyError::StorageFailure)
}

fn optional_string(value: &Value) -> Option<String> {
    value.as_str().map(|value| value.to_owned())
}

fn parse_resource_type(value: &Value) -> Result<OntologyResourceType, OntologyError> {
    match required_string(value)?.as_str() {
        "class" => Ok(OntologyResourceType::Class),
        "property" => Ok(OntologyResourceType::Property),
        "domain" => Ok(OntologyResourceType::Domain),
        "range" => Ok(OntologyResourceType::Range),
        "constraint" => Ok(OntologyResourceType::Constraint),
        "shape" => Ok(OntologyResourceType::Shape),
        _ => Err(OntologyError::StorageFailure),
    }
}

fn ontology_domain_key(domain: &OntologyDomain) -> String {
    match domain {
        OntologyDomain::Operations => "operations".to_owned(),
        OntologyDomain::Development => "development".to_owned(),
        OntologyDomain::Test => "test".to_owned(),
        OntologyDomain::Design => "design".to_owned(),
        OntologyDomain::Custom(value) => value.clone(),
    }
}
