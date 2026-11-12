use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct IdentityInput {
    pub(crate) label: String,
    pub(crate) git_name: String,
    pub(crate) email: String,
    #[serde(default)]
    pub(crate) signing_key: Option<String>,
    #[serde(default)]
    pub(crate) signing_format: Option<String>,
    #[serde(default)]
    pub(crate) ssh_key: Option<String>,
    #[serde(default = "default_signing_required")]
    pub(crate) signing_required: bool,
}

fn default_signing_required() -> bool {
    true
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
    pub(crate) signing_key: Option<String>,
    pub(crate) signing_format: Option<String>,
    pub(crate) ssh_key: Option<String>,
    pub(crate) signing_required: bool,
}

#[derive(Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum IdentityError {
    InvalidLabel,
    InvalidName,
    InvalidEmail,
    InvalidSigningKey,
    InvalidSshKey,
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
        let label = self.label.trim().to_owned();
        let git_name = self.git_name.trim().to_owned();
        let email = self.email.trim().to_owned();

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

        let signing_key = match self.signing_key.map(|k| k.trim().to_owned()) {
            Some(key) if !key.is_empty() => {
                if key.chars().count() > 500 || key.chars().any(char::is_control) {
                    return Err(IdentityError::InvalidSigningKey);
                }
                Some(key)
            }
            _ => None,
        };

        let signing_format = if signing_key.is_some() {
            let format = match self.signing_format.as_deref().map(str::trim) {
                Some("openpgp") => "openpgp".to_owned(),
                _ => "ssh".to_owned(),
            };
            Some(format)
        } else {
            None
        };

        let ssh_key = match self.ssh_key.map(|k| k.trim().to_owned()) {
            Some(key) if !key.is_empty() => {
                if key.chars().count() > 500 || key.chars().any(char::is_control) {
                    return Err(IdentityError::InvalidSshKey);
                }
                Some(key)
            }
            _ => None,
        };

        let signing_required = self.signing_required && signing_key.is_some();
        Ok(Self {
            label,
            git_name,
            email,
            signing_key,
            signing_format,
            ssh_key,
            signing_required,
        })
    }
}
