use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct IdentityInput {
    pub(crate) label: String,
    pub(crate) git_name: String,
    pub(crate) email: String,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Identity {
    pub(crate) id: i64,
    pub(crate) label: String,
    pub(crate) git_name: String,
    pub(crate) email: String,
    pub(crate) is_default: bool,
    pub(crate) revision: i64,
    pub(crate) assignment_count: i64,
}

#[derive(Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum IdentityError {
    InvalidLabel,
    InvalidName,
    InvalidEmail,
    Conflict,
    InUse,
    StorageUnavailable,
    GitUnavailable,
    GitReadFailed,
    GitIdentityMissing,
}

fn valid_text(value: &str, max: usize) -> bool {
    !value.is_empty() && value.chars().count() <= max && !value.chars().any(char::is_control)
}

impl IdentityInput {
    pub(crate) fn validate(self) -> Result<Self, IdentityError> {
        let Self {
            label,
            git_name,
            email,
        } = Self {
            label: self.label.trim().into(),
            git_name: self.git_name.trim().into(),
            email: self.email.trim().into(),
        };

        if !valid_text(&label, 80) {
            return Err(IdentityError::InvalidLabel);
        }
        if !valid_text(&git_name, 120) || git_name.contains(['<', '>']) {
            return Err(IdentityError::InvalidName);
        }
        let valid_email_parts = email.split_once('@').is_some_and(|(local, domain)| {
            !local.is_empty() && !domain.is_empty() && !domain.contains('@')
        });
        if !valid_text(&email, 254)
            || !valid_email_parts
            || email.contains(['<', '>'])
            || email.chars().any(char::is_whitespace)
        {
            return Err(IdentityError::InvalidEmail);
        }

        Ok(Self {
            label,
            git_name,
            email,
        })
    }
}
