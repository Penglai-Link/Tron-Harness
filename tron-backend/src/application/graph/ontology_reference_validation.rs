use super::OntologyStore;
use crate::domain::graph::{
    OntologyError, OntologyRangeTarget, OntologyResourceType, OntologyShaclShape,
    ResolvedGraphScope,
};

pub(super) fn missing_resource(
    resource_type: OntologyResourceType,
    resource_code: String,
) -> OntologyError {
    OntologyError::MissingOntologyResource {
        resource_type: resource_type.as_str(),
        resource_code,
    }
}

pub(super) fn ensure_code_exists(
    resource_type: OntologyResourceType,
    resource_code: &str,
    exists: impl FnOnce(&str) -> Result<bool, OntologyError>,
) -> Result<(), OntologyError> {
    if exists(resource_code)? {
        return Ok(());
    }

    Err(missing_resource(resource_type, resource_code.to_owned()))
}

pub(super) fn ensure_codes_exist(
    store: &dyn OntologyStore,
    scope: &ResolvedGraphScope,
    resource_type: OntologyResourceType,
    resource_codes: &[String],
) -> Result<(), OntologyError> {
    let references = resource_codes
        .iter()
        .map(|code| (resource_type, code.clone()))
        .collect::<Vec<_>>();
    ensure_references_exist(store, scope, &references)
}
fn ensure_references_exist(
    store: &dyn OntologyStore,
    scope: &ResolvedGraphScope,
    references: &[(OntologyResourceType, String)],
) -> Result<(), OntologyError> {
    if references.is_empty() {
        return Ok(());
    }
    let present = store.resources_present(scope, references)?;
    if present.len() != references.len() {
        return Err(OntologyError::StorageFailure);
    }
    for ((kind, code), present) in references.iter().zip(present) {
        if !present {
            return Err(missing_resource(*kind, code.clone()));
        }
    }
    Ok(())
}

pub(super) fn ensure_range_target_exists(
    store: &dyn OntologyStore,
    scope: &ResolvedGraphScope,
    target: &OntologyRangeTarget,
) -> Result<(), OntologyError> {
    if let OntologyRangeTarget::Class { class_code } = target {
        ensure_code_exists(OntologyResourceType::Class, class_code, |code| {
            Ok(store.get_class(scope, code)?.is_some())
        })?;
    }

    Ok(())
}

pub(super) fn ensure_shape_references_exist(
    store: &dyn OntologyStore,
    scope: &ResolvedGraphScope,
    shape: &OntologyShaclShape,
) -> Result<(), OntologyError> {
    let mut references = vec![(OntologyResourceType::Class, shape.target_class_code.clone())];
    for property in &shape.property_constraints {
        references.push((
            OntologyResourceType::Property,
            property.property_code.clone(),
        ));
        references.extend(
            property
                .constraint_codes
                .iter()
                .map(|code| (OntologyResourceType::Constraint, code.clone())),
        );
    }
    ensure_references_exist(store, scope, &references)
}
