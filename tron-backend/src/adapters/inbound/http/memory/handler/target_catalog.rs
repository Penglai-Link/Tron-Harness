#[derive(Debug, Clone)]
struct TargetScope {
    public_scope: String,
    scope_code: String,
}

impl TargetScope {
    fn new(public_scope: &str, scope_code: &str) -> Self {
        Self {
            public_scope: public_scope.into(),
            scope_code: scope_code.into(),
        }
    }

    fn work_domain_code(&self) -> Option<String> {
        (self.public_scope == "WORK_DOMAIN").then(|| self.scope_code.clone())
    }
}

impl Default for TargetScope {
    fn default() -> Self {
        Self {
            public_scope: String::new(),
            scope_code: String::new(),
        }
    }
}

#[derive(Debug, Clone)]
struct ResponseCatalog {
    domains: Vec<MemoryWorkDomain>,
    user_code: String,
    user_label: String,
    workspace_code: String,
    workspace_label: String,
}

impl ResponseCatalog {
    fn target_for(&self, public_scope: &str, scope_code: &str) -> ResolvedTarget {
        match public_scope {
            "USER" => ResolvedTarget {
                id: "user:current".into(),
                label: self.user_label.clone(),
                code: self.user_code.clone(),
                scope: "user".into(),
            },
            "WORK_DOMAIN" => {
                let label = self
                    .domains
                    .iter()
                    .find(|domain| domain.domain_code == scope_code)
                    .map(|domain| domain.name.clone())
                    .unwrap_or_else(|| scope_code.to_string());
                ResolvedTarget {
                    id: format!("domain:{scope_code}"),
                    label,
                    code: scope_code.to_string(),
                    scope: "domain".into(),
                }
            }
            _ => ResolvedTarget {
                id: "space:current".into(),
                label: self.workspace_label.clone(),
                code: self.workspace_code.clone(),
                scope: "space".into(),
            },
        }
    }
}

#[derive(Debug, Clone)]
struct ResolvedTarget {
    id: String,
    label: String,
    code: String,
    scope: String,
}
