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
    #[serde(skip)]
    pub credential_ref: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AccountStatus {
    account: Option<Account>,
    state: &'static str,
    cleanup_pending: bool,
}

pub(super) fn now() -> Result<i64, GithubError> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .map_err(|_| GithubError::Storage)
}

fn lock(app: &Path) -> Result<File, GithubError> {
    native_file::app_lock(&app.join("github-account.lock")).map_err(|_| GithubError::Busy)
}

pub(super) fn random_id() -> Result<String, GithubError> {
    let mut bytes = [0; 24];
    getrandom::fill(&mut bytes).map_err(|_| GithubError::Unavailable)?;
    Ok(hex::encode(bytes))
}

async fn row(pool: &SqlitePool) -> Result<Option<Account>, GithubError> {
    Ok(sqlx::query_as("SELECT github_id, login, expires_at, verified_at, credential_ref FROM github_accounts WHERE slot = 1 AND client_id = ?")
        .bind(CLIENT_ID).fetch_optional(pool).await?)
}

async fn reconcile(pool: &SqlitePool, vault: &Vault) -> Result<bool, GithubError> {
    let references: Vec<String> = sqlx::query_scalar(
        "SELECT reference FROM github_credentials WHERE reference NOT IN (SELECT credential_ref FROM github_accounts) LIMIT 8"
    ).fetch_all(pool).await?;
    for reference in references {
        if vault.delete(&reference).await.is_ok() {
            sqlx::query("DELETE FROM github_credentials WHERE reference = ?")
                .bind(reference)
                .execute(pool)
                .await?;
        }
    }
    let remaining: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM github_credentials WHERE reference NOT IN (SELECT credential_ref FROM github_accounts))")
        .fetch_one(pool).await?;
    Ok(remaining)
}

async fn read(vault: &Vault, account: &Account) -> Result<Credential, GithubError> {
    let bytes = vault.read(&account.credential_ref).await?;
    let credential: Credential =
        serde_json::from_slice(&bytes).map_err(|_| GithubError::SecureStore)?;
    if credential.github_id != account.github_id
        || credential.client_id != CLIENT_ID
        || credential.expires_at != account.expires_at
    {
        return Err(GithubError::SecureStore);
    }
    Ok(credential)
}

pub(super) async fn status(pool: &SqlitePool, app: &Path) -> Result<AccountStatus, GithubError> {
    let _lock = lock(app)?;
    let vault = Vault::new(app);
    let cleanup_pending = reconcile(pool, &vault).await?;
    let account = row(pool).await?;
    let state = match &account {
        Some(account) if read(&vault, account).await.is_err() => "unavailable",
        Some(account) if account.expires_at <= now()? => "expired",
        Some(_) => "connected",
        None => "absent",
    };
    Ok(AccountStatus {
        account,
        state,
        cleanup_pending,
    })
}

pub(super) async fn credentials(
    pool: &SqlitePool,
    app: &Path,
) -> Result<(Account, Credential), GithubError> {
    let _lock = lock(app)?;
    let account = row(pool).await?.ok_or(GithubError::NotConnected)?;
    if account.expires_at <= now()? {
        return Err(GithubError::Expired);
    }
    let credential = read(&Vault::new(app), &account).await?;
    Ok((account, credential))
}

pub(super) async fn mark_verified(
    pool: &SqlitePool,
    account: &Account,
    login: &str,
) -> Result<(), GithubError> {
    let updated = sqlx::query("UPDATE github_accounts SET login = ?, verified_at = ? WHERE credential_ref = ? AND github_id = ?")
        .bind(login).bind(now()?).bind(&account.credential_ref).bind(account.github_id)
        .execute(pool).await?;
    if updated.rows_affected() != 1 {
        return Err(GithubError::Stale);
    }
    Ok(())
}

pub(super) async fn save(
    pool: &SqlitePool,
    app: &Path,
    login: &str,
    credential: &Credential,
) -> Result<(), GithubError> {
    let _lock = lock(app)?;
    let vault = Vault::new(app);
    reconcile(pool, &vault).await?;
    let old = row(pool).await?;
    if old
        .as_ref()
        .is_some_and(|account| account.github_id != credential.github_id)
    {
        return Err(GithubError::DifferentAccount);
    }
    let reference = random_id()?;
    sqlx::query("INSERT INTO github_credentials (reference, state) VALUES (?, 'pending')")
        .bind(&reference)
        .execute(pool)
        .await?;
    let bytes = Zeroizing::new(serde_json::to_vec(credential).map_err(|_| GithubError::Protocol)?);
    vault.write(&reference, &bytes).await?;
    if *vault.read(&reference).await? != *bytes {
        return Err(GithubError::SecureStore);
    }
    let mut transaction = pool.begin().await?;
    sqlx::query("INSERT INTO github_accounts (slot, github_id, client_id, login, expires_at, credential_ref, verified_at)
        VALUES (1, ?, ?, ?, ?, ?, ?) ON CONFLICT(slot) DO UPDATE SET github_id = excluded.github_id,
        client_id = excluded.client_id, login = excluded.login, expires_at = excluded.expires_at,
        credential_ref = excluded.credential_ref, verified_at = excluded.verified_at")
        .bind(credential.github_id).bind(&credential.client_id).bind(login).bind(credential.expires_at)
        .bind(&reference).bind(now()?).execute(&mut *transaction).await?;
    sqlx::query("UPDATE github_credentials SET state = 'active' WHERE reference = ?")
        .bind(&reference)
        .execute(&mut *transaction)
        .await?;
    if let Some(old) = old {
        sqlx::query("UPDATE github_credentials SET state = 'retiring' WHERE reference = ?")
            .bind(old.credential_ref)
            .execute(&mut *transaction)
            .await?;
    }
    transaction.commit().await?;
    reconcile(pool, &vault).await?;
    Ok(())
}
