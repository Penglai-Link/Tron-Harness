use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};

use crate::shared::time::current_timestamp;

use super::MemoryError;

pub const DEFAULT_WORK_DOMAIN_CODE: &str = "main-domain";

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryWorkDomain {
    pub domain_code: String,
    pub name: String,
    pub description: Option<String>,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryWorkDomainQuery {
    pub search: Option<String>,
    pub include_disabled: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateMemoryWorkDomainCommand {
    pub domain_code: String,
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateMemoryWorkDomainCommand {
    pub name: Option<String>,
    pub description: Option<String>,
    pub status: Option<String>,
}

pub trait MemoryWorkDomainRepository: Send {
    fn list_domains(&mut self) -> Result<Vec<MemoryWorkDomain>, MemoryError>;
    fn create_domain(&mut self, domain: &MemoryWorkDomain) -> Result<(), MemoryError>;
    fn update_domain(&mut self, domain: &MemoryWorkDomain) -> Result<(), MemoryError>;
}

#[derive(Clone)]
pub struct MemoryWorkDomainService {
    repository: Arc<Mutex<Box<dyn MemoryWorkDomainRepository>>>,
}

impl MemoryWorkDomainService {
    pub fn new(repository: impl MemoryWorkDomainRepository + 'static) -> Self {
        Self {
            repository: Arc::new(Mutex::new(Box::new(repository))),
        }
    }

    pub fn list_domains(
        &self,
        query: MemoryWorkDomainQuery,
    ) -> Result<Vec<MemoryWorkDomain>, MemoryError> {
        let search = normalize_optional(query.search.as_deref()).map(|value| value.to_lowercase());
        let include_disabled = query.include_disabled.unwrap_or(false);
        let mut domains = self
            .repository
            .lock()
            .expect("memory work domain repository mutex poisoned")
            .list_domains()?;
        domains.retain(|domain| {
            (include_disabled || domain.status == "ACTIVE")
                && search.as_ref().is_none_or(|search| {
                    domain.domain_code.to_lowercase().contains(search)
                        || domain.name.to_lowercase().contains(search)
                        || domain
                            .description
                            .as_deref()
                            .unwrap_or_default()
                            .to_lowercase()
                            .contains(search)
                })
        });
        Ok(domains)
    }

    pub fn active_domains(&self) -> Result<Vec<MemoryWorkDomain>, MemoryError> {
        self.list_domains(MemoryWorkDomainQuery {
            search: None,
            include_disabled: Some(false),
        })
    }

    pub fn get_domain(&self, domain_code: &str) -> Result<MemoryWorkDomain, MemoryError> {
        let domain_code = normalize_domain_code(domain_code)?;
        self.repository
            .lock()
            .expect("memory work domain repository mutex poisoned")
            .list_domains()?
            .into_iter()
            .find(|domain| domain.domain_code == domain_code)
            .ok_or_else(|| MemoryError::NotFound("memory work domain was not found".into()))
    }

    pub fn ensure_active_domain(&self, domain_code: &str) -> Result<MemoryWorkDomain, MemoryError> {
        let domain = self.get_domain(domain_code)?;
        if domain.status == "ACTIVE" {
            Ok(domain)
        } else {
            Err(MemoryError::Validation(
                "memory work domain is disabled".into(),
            ))
        }
    }

    pub fn create_domain(
        &self,
        command: CreateMemoryWorkDomainCommand,
    ) -> Result<MemoryWorkDomain, MemoryError> {
        let domain_code = normalize_domain_code(&command.domain_code)?;
        let name = normalize_required(&command.name, "name")?;
        if self
            .repository
            .lock()
            .expect("memory work domain repository mutex poisoned")
            .list_domains()?
            .iter()
            .any(|domain| domain.domain_code == domain_code)
        {
            return Err(MemoryError::Conflict(
                "memory work domain already exists".into(),
            ));
        }
        let timestamp = current_timestamp();
        let domain = MemoryWorkDomain {
            domain_code,
            name,
            description: normalize_optional(command.description.as_deref()),
            status: "ACTIVE".into(),
            created_at: timestamp.clone(),
            updated_at: timestamp,
        };
        self.repository
            .lock()
            .expect("memory work domain repository mutex poisoned")
            .create_domain(&domain)?;
        Ok(domain)
    }

    pub fn update_domain(
        &self,
        domain_code: &str,
        command: UpdateMemoryWorkDomainCommand,
    ) -> Result<MemoryWorkDomain, MemoryError> {
        let mut domain = self.get_domain(domain_code)?;
        if let Some(name) = command.name {
            domain.name = normalize_required(&name, "name")?;
        }
        if command.description.is_some() {
            domain.description = normalize_optional(command.description.as_deref());
        }
        if let Some(status) = command.status {
            domain.status = normalize_status(&status)?;
        }
        domain.updated_at = current_timestamp();
        self.repository
            .lock()
            .expect("memory work domain repository mutex poisoned")
            .update_domain(&domain)?;
        Ok(domain)
    }

    pub fn set_domain_status(
        &self,
        domain_code: &str,
        status: &str,
    ) -> Result<MemoryWorkDomain, MemoryError> {
        self.update_domain(
            domain_code,
            UpdateMemoryWorkDomainCommand {
                name: None,
                description: None,
                status: Some(status.into()),
            },
        )
    }
}

fn normalize_domain_code(value: &str) -> Result<String, MemoryError> {
    let value = normalize_required(value, "domainCode")?.to_lowercase();
    if value.len() > 64
        || !value.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.')
        })
    {
        return Err(MemoryError::Validation(
            "domainCode must use letters, digits, hyphen, underscore, or dot".into(),
        ));
    }
    Ok(value)
}

fn normalize_status(value: &str) -> Result<String, MemoryError> {
    match value.trim().to_ascii_uppercase().as_str() {
        "ACTIVE" => Ok("ACTIVE".into()),
        "DISABLED" => Ok("DISABLED".into()),
        _ => Err(MemoryError::Validation(
            "status must be ACTIVE or DISABLED".into(),
        )),
    }
}

fn normalize_required(value: &str, field: &str) -> Result<String, MemoryError> {
    normalize_optional(Some(value))
        .ok_or_else(|| MemoryError::Validation(format!("{field} is required")))
}

fn normalize_optional(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}
