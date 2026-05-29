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
    resource_type: OntologyResourceType,
    resource_codes: &[String],
    mut exists: impl FnMut(&str) -> Result<bool, OntologyError>,
) -> Result<(), OntologyError> {
    for resource_code in resource_codes {
        ensure_code_exists(resource_type, resource_code, &mut exists)?;
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
    ensure_code_exists(
        OntologyResourceType::Class,
        &shape.target_class_code,
        |code| Ok(store.get_class(scope, code)?.is_some()),
    )?;

    for property_constraint in &shape.property_constraints {
        ensure_code_exists(
            OntologyResourceType::Property,
            &property_constraint.property_code,
            |code| Ok(store.get_property(scope, code)?.is_some()),
        )?;
        ensure_codes_exist(
            OntologyResourceType::Constraint,
            &property_constraint.constraint_codes,
            |code| Ok(store.get_constraint(scope, code)?.is_some()),
        )?;
    }

    Ok(())
}
