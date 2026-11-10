use sqlx::{query, query_as, query_scalar, SqlitePool};

use crate::identity::{Identity, IdentityError, IdentityInput};

impl From<sqlx::Error> for IdentityError {
    fn from(error: sqlx::Error) -> Self {
        match error {
            sqlx::Error::RowNotFound => Self::Conflict,
            sqlx::Error::Database(error)
                if error.is_foreign_key_violation()
                    // SQLite implements ON DELETE RESTRICT as a constraint trigger.
                    || (error.code().as_deref() == Some("1811")
                        && error.message() == "FOREIGN KEY constraint failed") =>
            {
                Self::InUse
            }
            _ => Self::StorageUnavailable,
        }
    }
}

pub(crate) async fn list(pool: &SqlitePool) -> Result<Vec<Identity>, IdentityError> {
    Ok(query_as(
        "SELECT id, label, git_name, email, is_default, revision,
            (SELECT count(*) FROM identity_rules WHERE identity_id = identities.id) AS assignment_count,
            signing_key, signing_format, ssh_key
         FROM identities ORDER BY is_default DESC, label COLLATE NOCASE, id",
    )
    .fetch_all(pool)
    .await?)
}

pub(crate) async fn create(pool: &SqlitePool, input: IdentityInput) -> Result<i64, IdentityError> {
    let input = input.validate()?;
    Ok(query_scalar(
        "INSERT INTO identities (label, git_name, email, signing_key, signing_format, ssh_key) VALUES (?, ?, ?, ?, ?, ?) RETURNING id",
    )
    .bind(input.label)
    .bind(input.git_name)
    .bind(input.email)
    .bind(input.signing_key)
    .bind(input.signing_format)
    .bind(input.ssh_key)
    .fetch_one(pool)
    .await?)
}

pub(crate) async fn update(
    pool: &SqlitePool,
    id: i64,
    revision: i64,
    input: IdentityInput,
) -> Result<(), IdentityError> {
    let input = input.validate()?;
    query_scalar::<_, i64>(
        "UPDATE identities SET label = ?, git_name = ?, email = ?,
            signing_key = ?, signing_format = ?, ssh_key = ?,
            revision = revision + 1, updated_at = CURRENT_TIMESTAMP
         WHERE id = ? AND revision = ? RETURNING id",
    )
    .bind(input.label)
    .bind(input.git_name)
    .bind(input.email)
    .bind(input.signing_key)
    .bind(input.signing_format)
    .bind(input.ssh_key)
    .bind(id)
    .bind(revision)
    .fetch_one(pool)
    .await?;
    Ok(())
}

pub(crate) async fn duplicate(
    pool: &SqlitePool,
    id: i64,
    revision: i64,
) -> Result<i64, IdentityError> {
    Ok(query_scalar(
        "INSERT INTO identities (label, git_name, email, signing_key, signing_format, ssh_key)
         SELECT substr(label, 1, 72) || ' (copia)', git_name, email, signing_key, signing_format, ssh_key
         FROM identities WHERE id = ? AND revision = ? RETURNING id",
    )
    .bind(id)
    .bind(revision)
    .fetch_one(pool)
    .await?)
}

pub(crate) async fn set_default(
    pool: &SqlitePool,
    id: i64,
    revision: i64,
    enabled: bool,
) -> Result<(), IdentityError> {
    let mut transaction = pool.begin().await?;
    if enabled {
        query(
            "UPDATE identities SET is_default = 0, revision = revision + 1,
                updated_at = CURRENT_TIMESTAMP WHERE is_default = 1 AND id <> ?",
        )
        .bind(id)
        .execute(&mut *transaction)
        .await?;
    }
    query_scalar::<_, i64>(
        "UPDATE identities SET is_default = ?, revision = revision + 1,
            updated_at = CURRENT_TIMESTAMP WHERE id = ? AND revision = ? RETURNING id",
    )
    .bind(enabled)
    .bind(id)
    .bind(revision)
    .fetch_one(&mut *transaction)
    .await?;
    transaction.commit().await?;
    Ok(())
}

pub(crate) async fn delete(pool: &SqlitePool, id: i64, revision: i64) -> Result<(), IdentityError> {
    let mut transaction = pool.begin().await?;
    let removed: Option<i64> = query_scalar(
        "DELETE FROM identities WHERE id = ? AND revision = ? AND is_default = 0 RETURNING id",
    )
    .bind(id)
    .bind(revision)
    .fetch_optional(&mut *transaction)
    .await?;
    if removed.is_none() {
        let is_default: Option<bool> =
            query_scalar("SELECT is_default FROM identities WHERE id = ? AND revision = ?")
                .bind(id)
                .bind(revision)
                .fetch_optional(&mut *transaction)
                .await?;
        return Err(if is_default == Some(true) {
            IdentityError::InUse
        } else {
            IdentityError::Conflict
        });
    }
    transaction.commit().await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::storage::open;

    fn input(label: &str) -> IdentityInput {
        IdentityInput {
            label: label.into(),
            git_name: "Ana López".into(),
            email: "ana@example.test".into(),
            signing_key: None,
            signing_format: None,
            ssh_key: None,
        }
    }

    #[test]
    fn persists_edits_and_copies_with_the_same_email() {
        tauri::async_runtime::block_on(async {
            let folder = tempfile::tempdir().unwrap();
            let pool = open(folder.path()).await.unwrap();
            let id = create(&pool, input(" Personal ")).await.unwrap();
            update(&pool, id, 1, input("Trabajo")).await.unwrap();
            assert_eq!(
                update(&pool, id, 1, input("Antigua")).await,
                Err(IdentityError::Conflict)
            );
            let copy = duplicate(&pool, id, 2).await.unwrap();
            pool.close().await;
            let pool = open(folder.path()).await.unwrap();
            let profiles = list(&pool).await.unwrap();
            assert_eq!(profiles.len(), 2);
            assert_eq!(profiles[0].label, "Trabajo");
            assert_eq!(profiles[1].label, "Trabajo (copia)");
            assert_eq!(profiles[0].email, profiles[1].email);
            delete(&pool, copy, 1).await.unwrap();
            let next = create(&pool, input("Otro")).await.unwrap();
            assert!(next > copy);
            pool.close().await;
        });
    }

    #[test]
    fn switches_defaults_atomically_and_protects_dependencies() {
        tauri::async_runtime::block_on(async {
            let folder = tempfile::tempdir().unwrap();
            let pool = open(folder.path()).await.unwrap();
            let a = create(&pool, input("A")).await.unwrap();
            let b = create(&pool, input("B")).await.unwrap();
            set_default(&pool, a, 1, true).await.unwrap();
            assert_eq!(delete(&pool, a, 2).await, Err(IdentityError::InUse));
            assert_eq!(
                set_default(&pool, b, 2, true).await,
                Err(IdentityError::Conflict)
            );
            assert_eq!(list(&pool).await.unwrap()[0].id, a);
            set_default(&pool, b, 1, true).await.unwrap();
            assert_eq!(list(&pool).await.unwrap()[0].id, b);
            set_default(&pool, b, 2, false).await.unwrap();
            assert!(list(&pool)
                .await
                .unwrap()
                .iter()
                .all(|profile| !profile.is_default));

            query("CREATE TABLE fixture_rules (identity_id INTEGER REFERENCES identities(id) ON DELETE RESTRICT)")
                .execute(&pool)
                .await
                .unwrap();
            query("INSERT INTO fixture_rules VALUES (?)")
                .bind(a)
                .execute(&pool)
                .await
                .unwrap();
            assert_eq!(delete(&pool, a, 3).await, Err(IdentityError::InUse));
            pool.close().await;
        });
    }

    #[test]
    fn validates_input_before_persisting() {
        tauri::async_runtime::block_on(async {
            let folder = tempfile::tempdir().unwrap();
            let pool = open(folder.path()).await.unwrap();
            let mut profile = input(" ");
            assert_eq!(
                create(&pool, profile.clone()).await,
                Err(IdentityError::InvalidLabel)
            );
            profile.label = "Personal".into();
            profile.git_name = "Ana <X>".into();
            assert_eq!(
                create(&pool, profile.clone()).await,
                Err(IdentityError::InvalidName)
            );
            profile.git_name = "Ana".into();
            profile.email = "ana@@example.test".into();
            assert_eq!(
                create(&pool, profile).await,
                Err(IdentityError::InvalidEmail)
            );
            assert!(list(&pool).await.unwrap().is_empty());
            pool.close().await;
        });
    }
}
