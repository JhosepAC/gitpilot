use std::{
    fs::File,
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use zeroize::Zeroizing;

use super::{
    github::{GithubError, CLIENT_ID},
    github_vault::Vault,
    native_file,
};

#[derive(Deserialize, Serialize)]
pub(super) struct Credential {
    pub access_token: Zeroizing<String>,
    pub refresh_token: Zeroizing<String>,
    pub github_id: i64,
    pub expires_at: i64,
    pub refresh_expires_at: i64,
    pub client_id: String,
}

#[derive(sqlx::FromRow, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Account {
    pub github_id: i64,
    pub login: String,
    pub expires_at: i64,
    pub verified_at: i64,
    pub state: String,
    #[serde(skip)]
    pub credential_ref: String,
    #[serde(skip)]
    pub refresh_ref: Option<String>,
}

#[derive(Clone)]
pub(super) struct ExpectedAccount {
    pub id: i64,
    pub reference: String,
}

#[derive(sqlx::FromRow, Serialize)]
#[serde(rename_all = "camelCase")]
struct Revocation {
    github_id: i64,
    login: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AccountStatus {
    accounts: Vec<Account>,
    revocations: Vec<Revocation>,
    cleanup_pending: bool,
}

const COLUMNS: &str =
    "github_id, login, expires_at, verified_at, state, credential_ref, refresh_ref";
const ORPHANS: &str = "SELECT reference FROM github_credentials WHERE NOT EXISTS (
    SELECT 1 FROM github_accounts WHERE credential_ref = reference OR refresh_ref = reference)";

pub(super) fn now() -> Result<i64, GithubError> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .map_err(|_| GithubError::Storage)
}

pub(super) fn random_id() -> Result<String, GithubError> {
    let mut bytes = [0; 24];
    getrandom::fill(&mut bytes).map_err(|_| GithubError::Unavailable)?;
    Ok(hex::encode(bytes))
}

pub(super) struct Accounts<'a> {
    pool: &'a SqlitePool,
    vault: Vault,
    _lock: File,
}

impl<'a> Accounts<'a> {
    pub(super) fn lock(pool: &'a SqlitePool, app: &Path) -> Result<Self, GithubError> {
        let lock = native_file::app_lock(&app.join("github-account.lock"))
            .map_err(|_| GithubError::Busy)?;
        Ok(Self {
            pool,
            vault: Vault::new(app),
            _lock: lock,
        })
    }

    pub(super) async fn row(&self, id: i64) -> Result<Option<Account>, GithubError> {
        Ok(sqlx::query_as(&format!(
            "SELECT {COLUMNS} FROM github_accounts WHERE client_id = ? AND github_id = ?"
        ))
        .bind(CLIENT_ID)
        .bind(id)
        .fetch_optional(self.pool)
        .await?)
    }

    async fn reconcile(&self) -> Result<bool, GithubError> {
        let references: Vec<String> = sqlx::query_scalar(&format!("{ORPHANS} LIMIT 8"))
            .fetch_all(self.pool)
            .await?;
        for reference in references {
            if self.vault.delete(&reference).await.is_ok() {
                sqlx::query("DELETE FROM github_credentials WHERE reference = ?")
                    .bind(reference)
                    .execute(self.pool)
                    .await?;
            }
        }
        Ok(sqlx::query_scalar(&format!("SELECT EXISTS({ORPHANS})"))
            .fetch_one(self.pool)
            .await?)
    }

    pub(super) async fn status(&self) -> Result<AccountStatus, GithubError> {
        let cleanup_pending = self.reconcile().await?;
        let mut accounts: Vec<Account> = sqlx::query_as(&format!("SELECT {COLUMNS} FROM github_accounts WHERE client_id = ? ORDER BY lower(login), github_id"))
            .bind(CLIENT_ID).fetch_all(self.pool).await?;
        let time = now()?;
        for account in &mut accounts {
            if account.state == "active" {
                account.state = if self.read(account, &account.credential_ref).await.is_err() {
                    "unavailable"
                } else if account.expires_at <= time {
                    "expired"
                } else {
                    "connected"
                }
                .into();
            }
        }
        let revocations = sqlx::query_as("SELECT github_id, login FROM github_revocations WHERE client_id = ? ORDER BY lower(login), github_id")
            .bind(CLIENT_ID).fetch_all(self.pool).await?;
        Ok(AccountStatus {
            accounts,
            revocations,
            cleanup_pending,
        })
    }

    pub(super) async fn read(
        &self,
        account: &Account,
        reference: &str,
    ) -> Result<Credential, GithubError> {
        let bytes = self.vault.read(reference).await?;
        let credential: Credential =
            serde_json::from_slice(&bytes).map_err(|_| GithubError::SecureStore)?;
        if credential.github_id != account.github_id
            || credential.client_id != CLIENT_ID
            || (reference == account.credential_ref && credential.expires_at != account.expires_at)
        {
            return Err(GithubError::SecureStore);
        }
        Ok(credential)
    }

    pub(super) async fn mark_verified(
        &self,
        account: &Account,
        login: &str,
    ) -> Result<(), GithubError> {
        let updated = sqlx::query("UPDATE github_accounts SET login = ?, verified_at = ? WHERE client_id = ? AND github_id = ? AND credential_ref = ? AND state = 'active'")
            .bind(login).bind(now()?).bind(CLIENT_ID).bind(account.github_id).bind(&account.credential_ref)
            .execute(self.pool).await?;
        if updated.rows_affected() != 1 {
            return Err(GithubError::Stale);
        }
        Ok(())
    }

    async fn reserve(
        transaction: &mut sqlx::SqliteTransaction<'_>,
        id: i64,
    ) -> Result<String, GithubError> {
        let reference = random_id()?;
        sqlx::query("INSERT INTO github_credentials (reference, state, client_id, github_id) VALUES (?, 'pending', ?, ?)")
            .bind(&reference).bind(CLIENT_ID).bind(id).execute(&mut **transaction).await?;
        Ok(reference)
    }

    pub(super) async fn prepare_refresh(&self, account: &Account) -> Result<String, GithubError> {
        let mut transaction = self.pool.begin().await?;
        let reference = Self::reserve(&mut transaction, account.github_id).await?;
        let updated = sqlx::query("UPDATE github_accounts SET state = 'refreshing', refresh_ref = ? WHERE client_id = ? AND github_id = ? AND credential_ref = ? AND state = 'active'")
            .bind(&reference).bind(CLIENT_ID).bind(account.github_id).bind(&account.credential_ref)
            .execute(&mut *transaction).await?;
        if updated.rows_affected() != 1 {
            return Err(GithubError::Stale);
        }
        transaction.commit().await?;
        Ok(reference)
    }

    pub(super) async fn require_authorization(&self, id: i64) -> Result<(), GithubError> {
        sqlx::query("UPDATE github_accounts SET state = 'reauthorize', refresh_ref = NULL WHERE client_id = ? AND github_id = ?")
            .bind(CLIENT_ID).bind(id).execute(self.pool).await?;
        Ok(())
    }

    pub(super) async fn write(
        &self,
        reference: &str,
        credential: &Credential,
    ) -> Result<(), GithubError> {
        let bytes =
            Zeroizing::new(serde_json::to_vec(credential).map_err(|_| GithubError::Protocol)?);
        self.vault.write(reference, &bytes).await?;
        if *self.vault.read(reference).await? != *bytes {
            return Err(GithubError::SecureStore);
        }
        Ok(())
    }

    pub(super) async fn activate(
        &self,
        reference: &str,
        login: &str,
        credential: &Credential,
    ) -> Result<(), GithubError> {
        let previous = self.row(credential.github_id).await?;
        let mut transaction = self.pool.begin().await?;
        sqlx::query("INSERT INTO github_accounts (client_id, github_id, login, expires_at, credential_ref, verified_at)
            VALUES (?, ?, ?, ?, ?, ?) ON CONFLICT(client_id, github_id) DO UPDATE SET login = excluded.login,
            expires_at = excluded.expires_at, credential_ref = excluded.credential_ref, verified_at = excluded.verified_at,
            state = 'active', refresh_ref = NULL")
            .bind(CLIENT_ID).bind(credential.github_id).bind(login).bind(credential.expires_at).bind(reference).bind(now()?)
            .execute(&mut *transaction).await?;
        sqlx::query("UPDATE github_credentials SET state = 'active' WHERE reference = ?")
            .bind(reference)
            .execute(&mut *transaction)
            .await?;
        if let Some(previous) = previous {
            sqlx::query("UPDATE github_credentials SET state = 'retiring' WHERE reference = ?")
                .bind(previous.credential_ref)
                .execute(&mut *transaction)
                .await?;
        }
        sqlx::query("DELETE FROM github_revocations WHERE client_id = ? AND github_id = ?")
            .bind(CLIENT_ID)
            .bind(credential.github_id)
            .execute(&mut *transaction)
            .await?;
        transaction.commit().await?;
        Ok(())
    }

    pub(super) async fn save(
        &self,
        login: &str,
        credential: &Credential,
        expected: Option<&ExpectedAccount>,
    ) -> Result<(), GithubError> {
        let old = self.row(credential.github_id).await?;
        if let Some(expected) = expected {
            if expected.id != credential.github_id {
                return Err(GithubError::DifferentAccount);
            }
            if old
                .as_ref()
                .is_none_or(|old| old.credential_ref != expected.reference)
            {
                return Err(GithubError::Stale);
            }
        }
        if old.is_some_and(|account| account.state == "disconnecting") {
            return Err(GithubError::Busy);
        }
        let mut transaction = self.pool.begin().await?;
        let reference = Self::reserve(&mut transaction, credential.github_id).await?;
        transaction.commit().await?;
        self.write(&reference, credential).await?;
        self.activate(&reference, login, credential).await
    }

    pub(super) async fn disconnect(&self, id: i64) -> Result<AccountStatus, GithubError> {
        let Some(account) = self.row(id).await? else {
            return self.status().await;
        };
        sqlx::query("UPDATE github_accounts SET state = 'disconnecting' WHERE client_id = ? AND github_id = ?")
            .bind(CLIENT_ID).bind(id).execute(self.pool).await?;
        let references: Vec<String> = sqlx::query_scalar(
            "SELECT reference FROM github_credentials WHERE client_id = ? AND github_id = ?",
        )
        .bind(CLIENT_ID)
        .bind(id)
        .fetch_all(self.pool)
        .await?;
        for reference in references {
            self.vault.delete(&reference).await?;
        }
        let mut transaction = self.pool.begin().await?;
        sqlx::query("DELETE FROM github_accounts WHERE client_id = ? AND github_id = ?")
            .bind(CLIENT_ID)
            .bind(id)
            .execute(&mut *transaction)
            .await?;
        sqlx::query("DELETE FROM github_credentials WHERE client_id = ? AND github_id = ?")
            .bind(CLIENT_ID)
            .bind(id)
            .execute(&mut *transaction)
            .await?;
        sqlx::query("INSERT INTO github_revocations (client_id, github_id, login) VALUES (?, ?, ?) ON CONFLICT(client_id, github_id) DO UPDATE SET login = excluded.login")
            .bind(CLIENT_ID).bind(id).bind(&account.login).execute(&mut *transaction).await?;
        transaction.commit().await?;
        self.status().await
    }

    pub(super) async fn acknowledge_revocation(
        &self,
        id: i64,
    ) -> Result<AccountStatus, GithubError> {
        sqlx::query("DELETE FROM github_revocations WHERE client_id = ? AND github_id = ?")
            .bind(CLIENT_ID)
            .bind(id)
            .execute(self.pool)
            .await?;
        self.status().await
    }
}
