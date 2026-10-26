use std::{
    collections::BTreeSet,
    sync::{Arc, Mutex},
};

use crate::{
    application::graph::OntologyStore,
    domain::graph::{
        OntologyClass, OntologyConstraint, OntologyDomainResource, OntologyError, OntologyProperty,
        OntologyRangeResource, OntologyResource, OntologyResourceType, OntologySearchQuery,
        OntologyShaclShape, ResolvedGraphScope,
    },
};

use super::{
    class_resource, constraint_resource, domain_resource, property_resource, range_resource,
    shape_resource,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum StoreCall {
    CreateClass {
        scope: ResolvedGraphScope,
        class: OntologyClass,
    },
    GetClass {
        scope: ResolvedGraphScope,
        class_code: String,
    },
    UpdateClass {
        scope: ResolvedGraphScope,
        class: OntologyClass,
    },
    DeleteClass {
        scope: ResolvedGraphScope,
        class_code: String,
    },
    CreateProperty {
        scope: ResolvedGraphScope,
        property: OntologyProperty,
    },
    GetProperty {
        scope: ResolvedGraphScope,
        property_code: String,
    },
    UpdateProperty {
        scope: ResolvedGraphScope,
        property: OntologyProperty,
    },
    DeleteProperty {
        scope: ResolvedGraphScope,
        property_code: String,
    },
    CreateDomain {
        scope: ResolvedGraphScope,
        domain: OntologyDomainResource,
    },
    GetDomain {
        scope: ResolvedGraphScope,
        domain_code: String,
    },
    UpdateDomain {
        scope: ResolvedGraphScope,
        domain: OntologyDomainResource,
    },
    DeleteDomain {
        scope: ResolvedGraphScope,
        domain_code: String,
    },
    CreateRange {
        scope: ResolvedGraphScope,
        range: OntologyRangeResource,
    },
    GetRange {
        scope: ResolvedGraphScope,
        range_code: String,
    },
    UpdateRange {
        scope: ResolvedGraphScope,
        range: OntologyRangeResource,
    },
    DeleteRange {
        scope: ResolvedGraphScope,
        range_code: String,
    },
    CreateConstraint {
        scope: ResolvedGraphScope,
        constraint: OntologyConstraint,
    },
    GetConstraint {
        scope: ResolvedGraphScope,
        constraint_code: String,
    },
    UpdateConstraint {
        scope: ResolvedGraphScope,
        constraint: OntologyConstraint,
    },
    DeleteConstraint {
        scope: ResolvedGraphScope,
        constraint_code: String,
    },
    CreateShape {
        scope: ResolvedGraphScope,
        shape: OntologyShaclShape,
    },
    GetShape {
        scope: ResolvedGraphScope,
        shape_code: String,
    },
    UpdateShape {
        scope: ResolvedGraphScope,
        shape: OntologyShaclShape,
    },
    DeleteShape {
        scope: ResolvedGraphScope,
        shape_code: String,
    },
    Search {
        scope: ResolvedGraphScope,
        query: OntologySearchQuery,
    },
}

#[derive(Default)]
struct FakeStoreState {
    calls: Vec<StoreCall>,
    classes: BTreeSet<String>,
    properties: BTreeSet<String>,
    domains: BTreeSet<String>,
    ranges: BTreeSet<String>,
    constraints: BTreeSet<String>,
    shapes: BTreeSet<String>,
    next_class_result: Option<Result<OntologyClass, OntologyError>>,
    next_constraint_result: Option<Result<OntologyConstraint, OntologyError>>,
}

#[derive(Clone, Default)]
pub(crate) struct FakeStore {
    state: Arc<Mutex<FakeStoreState>>,
}

impl FakeStore {
    pub(crate) fn with_resources(
        classes: &[&str],
        properties: &[&str],
        domains: &[&str],
        ranges: &[&str],
        constraints: &[&str],
        shapes: &[&str],
    ) -> Self {
        let store = Self::default();
        {
            let mut state = store.state.lock().expect("store state mutex poisoned");
            state
                .classes
                .extend(classes.iter().map(|code| (*code).to_owned()));
            state
                .properties
                .extend(properties.iter().map(|code| (*code).to_owned()));
            state
                .domains
                .extend(domains.iter().map(|code| (*code).to_owned()));
            state
                .ranges
                .extend(ranges.iter().map(|code| (*code).to_owned()));
            state
                .constraints
                .extend(constraints.iter().map(|code| (*code).to_owned()));
            state
                .shapes
                .extend(shapes.iter().map(|code| (*code).to_owned()));
        }
        store
    }

    pub(crate) fn calls(&self) -> Vec<StoreCall> {
        self.state
            .lock()
            .expect("store state mutex poisoned")
            .calls
            .clone()
    }

    pub(crate) fn set_next_class_result(&self, result: Result<OntologyClass, OntologyError>) {
        self.state
            .lock()
            .expect("store state mutex poisoned")
            .next_class_result = Some(result);
    }

    pub(crate) fn set_next_constraint_result(
        &self,
        result: Result<OntologyConstraint, OntologyError>,
    ) {
        self.state
            .lock()
            .expect("store state mutex poisoned")
            .next_constraint_result = Some(result);
    }
}

impl OntologyStore for FakeStore {
    fn resources_present(
        &self,
        scope: &ResolvedGraphScope,
        references: &[(OntologyResourceType, String)],
    ) -> Result<Vec<bool>, OntologyError> {
        // Record logical checks for existing assertions, while reading the in-memory
        // fixture in one operation under one lock.
        let mut state = self.state.lock().expect("store state mutex poisoned");
        Ok(references
            .iter()
            .map(|(kind, code)| {
                let (present, call) = match kind {
                    OntologyResourceType::Class => (
                        state.classes.contains(code),
                        StoreCall::GetClass {
                            scope: scope.clone(),
                            class_code: code.clone(),
                        },
                    ),
                    OntologyResourceType::Property => (
                        state.properties.contains(code),
                        StoreCall::GetProperty {
                            scope: scope.clone(),
                            property_code: code.clone(),
                        },
                    ),
                    OntologyResourceType::Domain => (
                        state.domains.contains(code),
                        StoreCall::GetDomain {
                            scope: scope.clone(),
                            domain_code: code.clone(),
                        },
                    ),
                    OntologyResourceType::Range => (
                        state.ranges.contains(code),
                        StoreCall::GetRange {
                            scope: scope.clone(),
                            range_code: code.clone(),
                        },
                    ),
                    OntologyResourceType::Constraint => (
                        state.constraints.contains(code),
                        StoreCall::GetConstraint {
                            scope: scope.clone(),
                            constraint_code: code.clone(),
                        },
                    ),
                    OntologyResourceType::Shape => (
                        state.shapes.contains(code),
                        StoreCall::GetShape {
                            scope: scope.clone(),
                            shape_code: code.clone(),
                        },
                    ),
                };
                state.calls.push(call);
                present
            })
            .collect())
    }
    fn create_class(
        &self,
        scope: &ResolvedGraphScope,
        class: OntologyClass,
    ) -> Result<OntologyClass, OntologyError> {
        let mut state = self.state.lock().expect("store state mutex poisoned");
        state.calls.push(StoreCall::CreateClass {
            scope: scope.clone(),
            class: class.clone(),
        });
        if let Some(result) = state.next_class_result.take() {
            return result;
        }
        Ok(class_resource(&class.class_code))
    }

    fn get_class(
        &self,
        scope: &ResolvedGraphScope,
        class_code: &str,
    ) -> Result<Option<OntologyClass>, OntologyError> {
        let mut state = self.state.lock().expect("store state mutex poisoned");
        state.calls.push(StoreCall::GetClass {
            scope: scope.clone(),
            class_code: class_code.to_owned(),
        });
        Ok(state
            .classes
            .contains(class_code)
            .then(|| class_resource(class_code)))
    }

    fn update_class(
        &self,
        scope: &ResolvedGraphScope,
        class: OntologyClass,
    ) -> Result<OntologyClass, OntologyError> {
        let mut state = self.state.lock().expect("store state mutex poisoned");
        state.calls.push(StoreCall::UpdateClass {
            scope: scope.clone(),
            class: class.clone(),
        });
        if let Some(result) = state.next_class_result.take() {
            return result;
        }
        Ok(class_resource(&class.class_code))
    }

    fn delete_class(
        &self,
        scope: &ResolvedGraphScope,
        class_code: &str,
    ) -> Result<(), OntologyError> {
        let mut state = self.state.lock().expect("store state mutex poisoned");
        state.calls.push(StoreCall::DeleteClass {
            scope: scope.clone(),
            class_code: class_code.to_owned(),
        });
        if !state.classes.contains(class_code) {
            return Err(missing(OntologyResourceType::Class, class_code));
        }
        Ok(())
    }

    fn create_property(
        &self,
        scope: &ResolvedGraphScope,
        property: OntologyProperty,
    ) -> Result<OntologyProperty, OntologyError> {
        let mut state = self.state.lock().expect("store state mutex poisoned");
        state.calls.push(StoreCall::CreateProperty {
            scope: scope.clone(),
            property: property.clone(),
        });
        Ok(property_resource(&property.property_code))
    }

    fn get_property(
        &self,
        scope: &ResolvedGraphScope,
        property_code: &str,
    ) -> Result<Option<OntologyProperty>, OntologyError> {
        let mut state = self.state.lock().expect("store state mutex poisoned");
        state.calls.push(StoreCall::GetProperty {
            scope: scope.clone(),
            property_code: property_code.to_owned(),
        });
        Ok(state
            .properties
            .contains(property_code)
            .then(|| property_resource(property_code)))
    }

    fn update_property(
        &self,
        scope: &ResolvedGraphScope,
        property: OntologyProperty,
    ) -> Result<OntologyProperty, OntologyError> {
        let mut state = self.state.lock().expect("store state mutex poisoned");
        state.calls.push(StoreCall::UpdateProperty {
            scope: scope.clone(),
            property: property.clone(),
        });
        Ok(property_resource(&property.property_code))
    }

    fn delete_property(
        &self,
        scope: &ResolvedGraphScope,
        property_code: &str,
    ) -> Result<(), OntologyError> {
        let mut state = self.state.lock().expect("store state mutex poisoned");
        state.calls.push(StoreCall::DeleteProperty {
            scope: scope.clone(),
            property_code: property_code.to_owned(),
        });
        if !state.properties.contains(property_code) {
            return Err(missing(OntologyResourceType::Property, property_code));
        }
        Ok(())
    }

    fn create_domain(
        &self,
        scope: &ResolvedGraphScope,
        domain: OntologyDomainResource,
    ) -> Result<OntologyDomainResource, OntologyError> {
        let mut state = self.state.lock().expect("store state mutex poisoned");
        state.calls.push(StoreCall::CreateDomain {
            scope: scope.clone(),
            domain: domain.clone(),
        });
        Ok(domain_resource(&domain.domain_code))
    }

    fn get_domain(
        &self,
        scope: &ResolvedGraphScope,
        domain_code: &str,
    ) -> Result<Option<OntologyDomainResource>, OntologyError> {
        let mut state = self.state.lock().expect("store state mutex poisoned");
        state.calls.push(StoreCall::GetDomain {
            scope: scope.clone(),
            domain_code: domain_code.to_owned(),
        });
        Ok(state
            .domains
            .contains(domain_code)
            .then(|| domain_resource(domain_code)))
    }

    fn update_domain(
        &self,
        scope: &ResolvedGraphScope,
        domain: OntologyDomainResource,
    ) -> Result<OntologyDomainResource, OntologyError> {
        let mut state = self.state.lock().expect("store state mutex poisoned");
        state.calls.push(StoreCall::UpdateDomain {
            scope: scope.clone(),
            domain: domain.clone(),
        });
        Ok(domain_resource(&domain.domain_code))
    }

    fn delete_domain(
        &self,
        scope: &ResolvedGraphScope,
        domain_code: &str,
    ) -> Result<(), OntologyError> {
        let mut state = self.state.lock().expect("store state mutex poisoned");
        state.calls.push(StoreCall::DeleteDomain {
            scope: scope.clone(),
            domain_code: domain_code.to_owned(),
        });
        if !state.domains.contains(domain_code) {
            return Err(missing(OntologyResourceType::Domain, domain_code));
        }
        Ok(())
    }

    fn create_range(
        &self,
        scope: &ResolvedGraphScope,
        range: OntologyRangeResource,
    ) -> Result<OntologyRangeResource, OntologyError> {
        let mut state = self.state.lock().expect("store state mutex poisoned");
        state.calls.push(StoreCall::CreateRange {
            scope: scope.clone(),
            range: range.clone(),
        });
        Ok(range_resource(&range.range_code))
    }

    fn get_range(
        &self,
        scope: &ResolvedGraphScope,
        range_code: &str,
    ) -> Result<Option<OntologyRangeResource>, OntologyError> {
        let mut state = self.state.lock().expect("store state mutex poisoned");
        state.calls.push(StoreCall::GetRange {
            scope: scope.clone(),
            range_code: range_code.to_owned(),
        });
        Ok(state
            .ranges
            .contains(range_code)
            .then(|| range_resource(range_code)))
    }

    fn update_range(
        &self,
        scope: &ResolvedGraphScope,
        range: OntologyRangeResource,
    ) -> Result<OntologyRangeResource, OntologyError> {
        let mut state = self.state.lock().expect("store state mutex poisoned");
        state.calls.push(StoreCall::UpdateRange {
            scope: scope.clone(),
            range: range.clone(),
        });
        Ok(range_resource(&range.range_code))
    }

    fn delete_range(
        &self,
        scope: &ResolvedGraphScope,
        range_code: &str,
    ) -> Result<(), OntologyError> {
        let mut state = self.state.lock().expect("store state mutex poisoned");
        state.calls.push(StoreCall::DeleteRange {
            scope: scope.clone(),
            range_code: range_code.to_owned(),
        });
        if !state.ranges.contains(range_code) {
            return Err(missing(OntologyResourceType::Range, range_code));
        }
        Ok(())
    }

    fn create_constraint(
        &self,
        scope: &ResolvedGraphScope,
        constraint: OntologyConstraint,
    ) -> Result<OntologyConstraint, OntologyError> {
        let mut state = self.state.lock().expect("store state mutex poisoned");
        state.calls.push(StoreCall::CreateConstraint {
            scope: scope.clone(),
            constraint: constraint.clone(),
        });
        if let Some(result) = state.next_constraint_result.take() {
            return result;
        }
        Ok(constraint_resource(&constraint.constraint_code))
    }

    fn get_constraint(
        &self,
        scope: &ResolvedGraphScope,
        constraint_code: &str,
    ) -> Result<Option<OntologyConstraint>, OntologyError> {
        let mut state = self.state.lock().expect("store state mutex poisoned");
        state.calls.push(StoreCall::GetConstraint {
            scope: scope.clone(),
            constraint_code: constraint_code.to_owned(),
        });
        Ok(state
            .constraints
            .contains(constraint_code)
            .then(|| constraint_resource(constraint_code)))
    }

    fn update_constraint(
        &self,
        scope: &ResolvedGraphScope,
        constraint: OntologyConstraint,
    ) -> Result<OntologyConstraint, OntologyError> {
        let mut state = self.state.lock().expect("store state mutex poisoned");
        state.calls.push(StoreCall::UpdateConstraint {
            scope: scope.clone(),
            constraint: constraint.clone(),
        });
        if let Some(result) = state.next_constraint_result.take() {
            return result;
        }
        Ok(constraint_resource(&constraint.constraint_code))
    }

    fn delete_constraint(
        &self,
        scope: &ResolvedGraphScope,
        constraint_code: &str,
    ) -> Result<(), OntologyError> {
        let mut state = self.state.lock().expect("store state mutex poisoned");
        state.calls.push(StoreCall::DeleteConstraint {
            scope: scope.clone(),
            constraint_code: constraint_code.to_owned(),
        });
        if !state.constraints.contains(constraint_code) {
            return Err(missing(OntologyResourceType::Constraint, constraint_code));
        }
        Ok(())
    }

    fn create_shape(
        &self,
        scope: &ResolvedGraphScope,
        shape: OntologyShaclShape,
    ) -> Result<OntologyShaclShape, OntologyError> {
        let mut state = self.state.lock().expect("store state mutex poisoned");
        state.calls.push(StoreCall::CreateShape {
            scope: scope.clone(),
            shape: shape.clone(),
        });
        Ok(shape_resource(&shape.shape_code))
    }

    fn get_shape(
        &self,
        scope: &ResolvedGraphScope,
        shape_code: &str,
    ) -> Result<Option<OntologyShaclShape>, OntologyError> {
        let mut state = self.state.lock().expect("store state mutex poisoned");
        state.calls.push(StoreCall::GetShape {
            scope: scope.clone(),
            shape_code: shape_code.to_owned(),
        });
        Ok(state
            .shapes
            .contains(shape_code)
            .then(|| shape_resource(shape_code)))
    }

    fn update_shape(
        &self,
        scope: &ResolvedGraphScope,
        shape: OntologyShaclShape,
    ) -> Result<OntologyShaclShape, OntologyError> {
        let mut state = self.state.lock().expect("store state mutex poisoned");
        state.calls.push(StoreCall::UpdateShape {
            scope: scope.clone(),
            shape: shape.clone(),
        });
        Ok(shape_resource(&shape.shape_code))
    }

    fn delete_shape(
        &self,
        scope: &ResolvedGraphScope,
        shape_code: &str,
    ) -> Result<(), OntologyError> {
        let mut state = self.state.lock().expect("store state mutex poisoned");
        state.calls.push(StoreCall::DeleteShape {
            scope: scope.clone(),
            shape_code: shape_code.to_owned(),
        });
        if !state.shapes.contains(shape_code) {
            return Err(missing(OntologyResourceType::Shape, shape_code));
        }
        Ok(())
    }

    fn search(
        &self,
        scope: &ResolvedGraphScope,
        query: OntologySearchQuery,
    ) -> Result<Vec<OntologyResource>, OntologyError> {
        self.state
            .lock()
            .expect("store state mutex poisoned")
            .calls
            .push(StoreCall::Search {
                scope: scope.clone(),
                query,
            });
        Ok(Vec::new())
    }
}

fn missing(resource_type: OntologyResourceType, resource_code: &str) -> OntologyError {
    OntologyError::MissingOntologyResource {
        resource_type: resource_type.as_str(),
        resource_code: resource_code.to_owned(),
    }
}
