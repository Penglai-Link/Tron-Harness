use std::collections::BTreeSet;

use super::OntologyError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OntologyDomain {
    Operations,
    Development,
    Test,
    Design,
    Custom(String),
}

impl OntologyDomain {
    fn validate(self) -> Result<Self, OntologyError> {
        match self {
            Self::Custom(value) => Ok(Self::Custom(normalize_domain_key(&value)?)),
            other => Ok(other),
        }
    }

    fn key(&self) -> String {
        match self {
            Self::Operations => "operations".to_owned(),
            Self::Development => "development".to_owned(),
            Self::Test => "test".to_owned(),
            Self::Design => "design".to_owned(),
            Self::Custom(value) => value.clone(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OntologyDataType {
    Text,
    Integer,
    Decimal,
    Boolean,
    Date,
    DateTime,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OntologyRangeTarget {
    Class { class_code: String },
    DataType(OntologyDataType),
}

impl OntologyRangeTarget {
    fn validate(self) -> Result<Self, OntologyError> {
        match self {
            Self::Class { class_code } => Ok(Self::Class {
                class_code: normalize_ontology_code(&class_code)?,
            }),
            Self::DataType(data_type) => Ok(Self::DataType(data_type)),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OntologyResourceType {
    Class,
    Property,
    Domain,
    Range,
    Constraint,
    Shape,
}

impl OntologyResourceType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Class => "class",
            Self::Property => "property",
            Self::Domain => "domain",
            Self::Range => "range",
            Self::Constraint => "constraint",
            Self::Shape => "shape",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OntologyClass {
    pub class_code: String,
    pub ontology_domain: OntologyDomain,
    pub label: String,
    pub description: Option<String>,
    pub parent_class_codes: Vec<String>,
}

impl OntologyClass {
    pub fn validate(self) -> Result<Self, OntologyError> {
        let class_code = normalize_ontology_code(&self.class_code)?;
        let parent_class_codes = normalize_code_list(self.parent_class_codes)?;
        if parent_class_codes.iter().any(|code| code == &class_code) {
            return Err(OntologyError::InconsistentOntologyResource);
        }

        Ok(Self {
            class_code,
            ontology_domain: self.ontology_domain.validate()?,
            label: normalize_label(&self.label)?,
            description: normalize_optional_text(self.description),
            parent_class_codes,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OntologyDomainResource {
    pub domain_code: String,
    pub ontology_domain: OntologyDomain,
    pub label: String,
    pub description: Option<String>,
    pub class_code: String,
}

impl OntologyDomainResource {
    pub fn validate(self) -> Result<Self, OntologyError> {
        Ok(Self {
            domain_code: normalize_ontology_code(&self.domain_code)?,
            ontology_domain: self.ontology_domain.validate()?,
            label: normalize_label(&self.label)?,
            description: normalize_optional_text(self.description),
            class_code: normalize_ontology_code(&self.class_code)?,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OntologyRangeResource {
    pub range_code: String,
    pub ontology_domain: OntologyDomain,
    pub label: String,
    pub description: Option<String>,
    pub target: OntologyRangeTarget,
}

impl OntologyRangeResource {
    pub fn validate(self) -> Result<Self, OntologyError> {
        Ok(Self {
            range_code: normalize_ontology_code(&self.range_code)?,
            ontology_domain: self.ontology_domain.validate()?,
            label: normalize_label(&self.label)?,
            description: normalize_optional_text(self.description),
            target: self.target.validate()?,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OntologyProperty {
    pub property_code: String,
    pub ontology_domain: OntologyDomain,
    pub label: String,
    pub description: Option<String>,
    pub domain_codes: Vec<String>,
    pub range_codes: Vec<String>,
}

impl OntologyProperty {
    pub fn validate(self) -> Result<Self, OntologyError> {
        let domain_codes = normalize_code_list(self.domain_codes)?;
        let range_codes = normalize_code_list(self.range_codes)?;
        if domain_codes.is_empty() || range_codes.is_empty() {
            return Err(OntologyError::InvalidOntologyResource);
        }

        Ok(Self {
            property_code: normalize_ontology_code(&self.property_code)?,
            ontology_domain: self.ontology_domain.validate()?,
            label: normalize_label(&self.label)?,
            description: normalize_optional_text(self.description),
            domain_codes,
            range_codes,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OntologyConstraint {
    pub constraint_code: String,
    pub ontology_domain: OntologyDomain,
    pub label: String,
    pub description: Option<String>,
    pub required: bool,
    pub min_count: Option<u32>,
    pub max_count: Option<u32>,
    pub allowed_range_codes: Vec<String>,
    pub pattern: Option<String>,
}

impl OntologyConstraint {
    pub fn validate(self) -> Result<Self, OntologyError> {
        let allowed_range_codes = normalize_code_list(self.allowed_range_codes)?;
        let pattern = normalize_optional_text(self.pattern);
        if matches!((self.min_count, self.max_count), (Some(min), Some(max)) if max < min) {
            return Err(OntologyError::InconsistentOntologyResource);
        }
        if !self.required
            && self.min_count.is_none()
            && self.max_count.is_none()
            && allowed_range_codes.is_empty()
            && pattern.is_none()
        {
            return Err(OntologyError::InvalidOntologyResource);
        }

        Ok(Self {
            constraint_code: normalize_ontology_code(&self.constraint_code)?,
            ontology_domain: self.ontology_domain.validate()?,
            label: normalize_label(&self.label)?,
            description: normalize_optional_text(self.description),
            required: self.required,
            min_count: self.min_count,
            max_count: self.max_count,
            allowed_range_codes,
            pattern,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OntologyShapeProperty {
    pub property_code: String,
    pub constraint_codes: Vec<String>,
}

impl OntologyShapeProperty {
    fn validate(self) -> Result<Self, OntologyError> {
        let constraint_codes = normalize_code_list(self.constraint_codes)?;
        if constraint_codes.is_empty() {
            return Err(OntologyError::InvalidOntologyResource);
        }

        Ok(Self {
            property_code: normalize_ontology_code(&self.property_code)?,
            constraint_codes,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OntologyShaclShape {
    pub shape_code: String,
    pub ontology_domain: OntologyDomain,
    pub label: String,
    pub description: Option<String>,
    pub target_class_code: String,
    pub property_constraints: Vec<OntologyShapeProperty>,
}

impl OntologyShaclShape {
    pub fn validate(self) -> Result<Self, OntologyError> {
        let property_constraints = normalize_shape_properties(self.property_constraints)?;
        if property_constraints.is_empty() {
            return Err(OntologyError::InvalidOntologyResource);
        }

        Ok(Self {
            shape_code: normalize_ontology_code(&self.shape_code)?,
            ontology_domain: self.ontology_domain.validate()?,
            label: normalize_label(&self.label)?,
            description: normalize_optional_text(self.description),
            target_class_code: normalize_ontology_code(&self.target_class_code)?,
            property_constraints,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OntologyResource {
    Class(OntologyClass),
    Property(OntologyProperty),
    Domain(OntologyDomainResource),
    Range(OntologyRangeResource),
    Constraint(OntologyConstraint),
    Shape(OntologyShaclShape),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OntologyQueryLimit {
    value: usize,
}

impl OntologyQueryLimit {
    pub fn new(value: usize) -> Result<Self, OntologyError> {
        if value == 0 {
            return Err(OntologyError::InvalidOntologyQuery);
        }

        Ok(Self { value })
    }

    pub fn value(self) -> usize {
        self.value
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OntologySearchQuery {
    pub resource_types: Vec<OntologyResourceType>,
    pub ontology_domains: Vec<OntologyDomain>,
    pub codes: Vec<String>,
    pub label: Option<String>,
    pub target_class_code: Option<String>,
    pub limit: OntologyQueryLimit,
}

impl OntologySearchQuery {
    pub fn empty() -> Self {
        Self {
            resource_types: Vec::new(),
            ontology_domains: Vec::new(),
            codes: Vec::new(),
            label: None,
            target_class_code: None,
            limit: OntologyQueryLimit { value: 25 },
        }
    }

    pub fn validate(self) -> Result<Self, OntologyError> {
        let resource_types = normalize_resource_types(self.resource_types);
        let ontology_domains = normalize_search_domains(self.ontology_domains)?;
        let codes = normalize_code_list(self.codes)?;
        let label = normalize_optional_text(self.label);
        let target_class_code = match self.target_class_code {
            Some(value) => Some(normalize_ontology_code(&value)?),
            None => None,
        };

        if resource_types.is_empty()
            && ontology_domains.is_empty()
            && codes.is_empty()
            && label.is_none()
            && target_class_code.is_none()
        {
            return Err(OntologyError::EmptyOntologySearchQuery);
        }

        Ok(Self {
            resource_types,
            ontology_domains,
            codes,
            label,
            target_class_code,
            limit: self.limit,
        })
    }
}

pub fn normalize_ontology_code(value: &str) -> Result<String, OntologyError> {
    let value = value.trim();
    if value.is_empty()
        || value
            .chars()
            .any(|ch| !(ch.is_ascii_uppercase() || ch.is_ascii_digit() || ch == '_' || ch == '-'))
    {
        return Err(OntologyError::InvalidOntologyCode);
    }

    Ok(value.to_owned())
}

fn normalize_label(value: &str) -> Result<String, OntologyError> {
    let value = value.trim();
    if value.is_empty() {
        return Err(OntologyError::InvalidOntologyResource);
    }

    Ok(value.to_owned())
}

fn normalize_domain_key(value: &str) -> Result<String, OntologyError> {
    let value = value.trim();
    if value.is_empty()
        || value
            .chars()
            .any(|ch| !(ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '_' || ch == '-'))
    {
        return Err(OntologyError::InvalidOntologyDomain);
    }

    Ok(value.to_owned())
}

fn normalize_optional_text(value: Option<String>) -> Option<String> {
    value.and_then(|value| {
        let value = value.trim();
        (!value.is_empty()).then(|| value.to_owned())
    })
}

fn normalize_code_list(values: Vec<String>) -> Result<Vec<String>, OntologyError> {
    let mut normalized = BTreeSet::new();
    for value in values {
        normalized.insert(normalize_ontology_code(&value)?);
    }

    Ok(normalized.into_iter().collect())
}

fn normalize_shape_properties(
    values: Vec<OntologyShapeProperty>,
) -> Result<Vec<OntologyShapeProperty>, OntologyError> {
    let mut seen = BTreeSet::new();
    let mut normalized = Vec::new();
    for value in values {
        let value = value.validate()?;
        if !seen.insert(value.property_code.clone()) {
            return Err(OntologyError::InconsistentOntologyResource);
        }
        normalized.push(value);
    }

    Ok(normalized)
}

fn normalize_resource_types(
    resource_types: Vec<OntologyResourceType>,
) -> Vec<OntologyResourceType> {
    let mut normalized = Vec::new();
    for resource_type in resource_types {
        if !normalized.contains(&resource_type) {
            normalized.push(resource_type);
        }
    }

    normalized
}

fn normalize_search_domains(
    ontology_domains: Vec<OntologyDomain>,
) -> Result<Vec<OntologyDomain>, OntologyError> {
    let mut seen = BTreeSet::new();
    let mut normalized = Vec::new();
    for ontology_domain in ontology_domains {
        let ontology_domain = ontology_domain.validate()?;
        if seen.insert(ontology_domain.key()) {
            normalized.push(ontology_domain);
        }
    }

    Ok(normalized)
}
