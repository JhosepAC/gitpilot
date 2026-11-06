use std::path::Path;

use sqlx::{query, query_as, query_scalar, SqlitePool, SqliteTransaction};

use crate::repository::{
    Directory, DirectoryInventory, DiscoveredRepository, ExpectedIdentity, InventoryError,
    Repository,
};

impl From<sqlx::Error> for InventoryError {
    fn from(error: sqlx::Error) -> Self {
        match error {
            sqlx::Error::RowNotFound => Self::NotFound,
            sqlx::Error::Database(error) if error.is_foreign_key_violation() => Self::Conflict,
            _ => Self::StorageUnavailable,
        }
    }
}

pub(crate) async fn add(pool: &SqlitePool, path: &str) -> Result<i64, InventoryError> {
    query_scalar(
        "INSERT INTO directories (path) VALUES (?) ON CONFLICT(path) DO NOTHING RETURNING id",
    )
    .bind(path)
    .fetch_optional(pool)
    .await?
    .ok_or(InventoryError::AlreadyRegistered)
}

pub(crate) async fn directories(pool: &SqlitePool) -> Result<Vec<Directory>, InventoryError> {
    Ok(query_as(
        "SELECT d.id, d.path, d.revision, d.scan_partial, rule.identity_id
         FROM directories d LEFT JOIN identity_rules rule ON rule.directory_id = d.id ORDER BY d.path",
    )
    .fetch_all(pool)
    .await?)
}

pub(crate) async fn directory(pool: &SqlitePool, id: i64) -> Result<Directory, InventoryError> {
    Ok(query_as(
        "SELECT d.id, d.path, d.revision, d.scan_partial, rule.identity_id
         FROM directories d LEFT JOIN identity_rules rule ON rule.directory_id = d.id WHERE d.id = ?",
    )
    .bind(id)
    .fetch_one(pool)
    .await?)
}

pub(crate) async fn contents(
    pool: &SqlitePool,
    id: i64,
) -> Result<DirectoryInventory, InventoryError> {
    let exclusions =
        query_scalar("SELECT path FROM directory_exclusions WHERE directory_id = ? ORDER BY path")
            .bind(id)
            .fetch_all(pool)
            .await?;
    let repositories = query_as(
        "SELECT r.id, r.path, r.kind, r.git_dir, r.common_dir, r.revision, dr.present, rule.identity_id
         FROM directory_repositories dr JOIN repositories r ON r.id = dr.repository_id
         LEFT JOIN identity_rules rule ON rule.repository_id = r.id
         WHERE dr.directory_id = ? ORDER BY r.path",
    ).bind(id).fetch_all(pool).await?;
    Ok(DirectoryInventory {
        exclusions,
        repositories,
    })
}

async fn advance_directory(
    tx: &mut SqliteTransaction<'_>,
    id: i64,
    revision: i64,
) -> Result<(), InventoryError> {
    let changed =
        query("UPDATE directories SET revision = revision + 1 WHERE id = ? AND revision = ?")
            .bind(id)
            .bind(revision)
            .execute(&mut **tx)
            .await?
            .rows_affected();
    if changed == 1 {
        Ok(())
    } else {
        Err(InventoryError::Conflict)
    }
}

async fn prune(tx: &mut SqliteTransaction<'_>) -> Result<(), InventoryError> {
    query(
        "DELETE FROM repositories WHERE NOT EXISTS (
               SELECT 1 FROM directory_repositories WHERE repository_id = repositories.id
           )",
    )
    .execute(&mut **tx)
    .await?;
    Ok(())
}

pub(crate) async fn remove(
    pool: &SqlitePool,
    id: i64,
    revision: i64,
) -> Result<(), InventoryError> {
    let mut tx = pool.begin().await?;
    let removed = query("DELETE FROM directories WHERE id = ? AND revision = ?")
        .bind(id)
        .bind(revision)
        .execute(&mut *tx)
        .await?
        .rows_affected();
    if removed != 1 {
        return Err(InventoryError::Conflict);
    }
    prune(&mut tx).await?;
    tx.commit().await?;
    Ok(())
}

pub(crate) async fn set_exclusion(
    pool: &SqlitePool,
    directory: &Directory,
    path: &str,
    excluded: bool,
) -> Result<(), InventoryError> {
    if path == directory.path || !Path::new(path).starts_with(&directory.path) {
        return Err(InventoryError::OutsideDirectory);
    }
    let mut tx = pool.begin().await?;
    advance_directory(&mut tx, directory.id, directory.revision).await?;
    let sql = if excluded {
        "INSERT OR IGNORE INTO directory_exclusions (directory_id, path) VALUES (?, ?)"
    } else {
        "DELETE FROM directory_exclusions WHERE directory_id = ? AND path = ?"
    };
    query(sql)
        .bind(directory.id)
        .bind(path)
        .execute(&mut *tx)
        .await?;
    query("UPDATE directories SET scan_partial = NULL WHERE id = ?")
        .bind(directory.id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(())
}

pub(crate) async fn save_scan(
    pool: &SqlitePool,
    directory: &Directory,
    repos: Vec<DiscoveredRepository>,
    partial: bool,
) -> Result<(), InventoryError> {
    let mut tx = pool.begin().await?;
    advance_directory(&mut tx, directory.id, directory.revision).await?;
    if !partial {
        query("UPDATE directory_repositories SET present = 0 WHERE directory_id = ?")
            .bind(directory.id)
            .execute(&mut *tx)
            .await?;
    }
    for repo in repos {
        let id: i64 = query_scalar(
            "INSERT INTO repositories (path, kind, git_dir, common_dir) VALUES (?, ?, ?, ?)
             ON CONFLICT(path) DO UPDATE SET
                revision = repositories.revision + (repositories.kind <> excluded.kind OR
                    repositories.git_dir <> excluded.git_dir OR repositories.common_dir <> excluded.common_dir),
                kind = excluded.kind, git_dir = excluded.git_dir, common_dir = excluded.common_dir
             RETURNING id",
        ).bind(repo.path).bind(repo.kind).bind(repo.git_dir).bind(repo.common_dir).fetch_one(&mut *tx).await?;
        query(
            "INSERT INTO directory_repositories (directory_id, repository_id) VALUES (?, ?)
               ON CONFLICT(directory_id, repository_id) DO UPDATE SET present = 1",
        )
        .bind(directory.id)
        .bind(id)
        .execute(&mut *tx)
        .await?;
    }
    query("DELETE FROM directory_repositories WHERE directory_id = ? AND present = 0
           AND NOT EXISTS (SELECT 1 FROM identity_rules WHERE repository_id = directory_repositories.repository_id)")
        .bind(directory.id).execute(&mut *tx).await?;
    query("UPDATE directories SET scan_partial = ? WHERE id = ?")
        .bind(partial)
        .bind(directory.id)
        .execute(&mut *tx)
        .await?;
    prune(&mut tx).await?;
    tx.commit().await?;
    Ok(())
}

pub(crate) async fn repository(pool: &SqlitePool, id: i64) -> Result<Repository, InventoryError> {
    Ok(query_as(
        "SELECT r.id, r.path, r.kind, r.git_dir, r.common_dir, r.revision, rule.identity_id,
          EXISTS (SELECT 1 FROM directory_repositories dr WHERE dr.repository_id = r.id AND dr.present = 1) AS present
         FROM repositories r LEFT JOIN identity_rules rule ON rule.repository_id = r.id WHERE r.id = ?",
    ).bind(id).fetch_one(pool).await?)
}

pub(crate) async fn assign(
    pool: &SqlitePool,
    repository: bool,
    id: i64,
    revision: i64,
    identity: Option<i64>,
) -> Result<(), InventoryError> {
    let mut tx = pool.begin().await?;
    let (column, advance) = if repository {
        (
            "repository_id",
            "UPDATE repositories SET revision = revision + 1 WHERE id = ? AND revision = ?
             AND (? OR EXISTS (SELECT 1 FROM directory_repositories dr
                       WHERE dr.repository_id = repositories.id AND dr.present = 1))",
        )
    } else {
        (
            "directory_id",
            "UPDATE directories SET revision = revision + 1 WHERE id = ? AND revision = ?",
        )
    };
    let mut update = query(advance).bind(id).bind(revision);
    if repository {
        update = update.bind(identity.is_none());
    }
    if update.execute(&mut *tx).await?.rows_affected() != 1 {
        return Err(InventoryError::Conflict);
    }
    if let Some(identity) = identity {
        let statement = format!(
            "INSERT INTO identity_rules ({column}, identity_id) VALUES (?, ?)
             ON CONFLICT({column}) DO UPDATE SET identity_id = excluded.identity_id"
        );
        query(&statement)
            .bind(id)
            .bind(identity)
            .execute(&mut *tx)
            .await?;
    } else {
        query(&format!("DELETE FROM identity_rules WHERE {column} = ?"))
            .bind(id)
            .execute(&mut *tx)
            .await?;
        if repository {
            query("DELETE FROM directory_repositories WHERE repository_id = ? AND present = 0")
                .bind(id)
                .execute(&mut *tx)
                .await?;
            prune(&mut tx).await?;
        }
    }
    tx.commit().await?;
    Ok(())
}

pub(crate) async fn expected(
    pool: &SqlitePool,
    repo: &Repository,
) -> Result<Option<ExpectedIdentity>, InventoryError> {
    type Profile = (String, String, String);
    let (profile, source, directory) = if let Some(id) = repo.identity_id {
        let profile: Profile =
            query_as("SELECT label, git_name, email FROM identities WHERE id = ?")
                .bind(id)
                .fetch_one(pool)
                .await?;
        (profile, "repository", None)
    } else {
        let rules: Vec<(String, String, String, String)> = query_as(
            "SELECT d.path, i.label, i.git_name, i.email FROM directories d
             JOIN identity_rules rule ON rule.directory_id = d.id JOIN identities i ON i.id = rule.identity_id"
        ).fetch_all(pool).await?;
        if let Some((path, label, name, email)) = rules
            .into_iter()
            .filter(|(path, ..)| Path::new(&repo.path).starts_with(path))
            .max_by_key(|(path, ..)| Path::new(path).components().count())
        {
            ((label, name, email), "directory", Some(path))
        } else if repo.present {
            let profile: Option<Profile> =
                query_as("SELECT label, git_name, email FROM identities WHERE is_default = 1")
                    .fetch_optional(pool)
                    .await?;
            let Some(profile) = profile else {
                return Ok(None);
            };
            (profile, "default", None)
        } else {
            return Ok(None);
        }
    };
    Ok(Some(ExpectedIdentity {
        label: profile.0,
        git_name: profile.1,
        email: profile.2,
        source,
        directory,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        identity::IdentityInput,
        infrastructure::{identities, storage},
    };

    #[test]
    fn overlapping_roots_preserve_rules_and_resolve_precedence() {
        tauri::async_runtime::block_on(async {
            let temp = tempfile::tempdir().unwrap();
            let pool = storage::open(temp.path()).await.unwrap();
            let outer_path = temp.path().join("Proyectos");
            let inner_path = outer_path.join("Trabajo");
            let repo_path = inner_path.join("repo");
            let outer = add(&pool, outer_path.to_str().unwrap()).await.unwrap();
            let inner = add(&pool, inner_path.to_str().unwrap()).await.unwrap();
            assert_eq!(
                add(&pool, inner_path.to_str().unwrap()).await,
                Err(InventoryError::AlreadyRegistered)
            );
            let mut ids = Vec::new();
            for label in ["predeterminada", "general", "trabajo", "excepción"] {
                ids.push(
                    identities::create(
                        &pool,
                        IdentityInput {
                            label: label.into(),
                            git_name: "Fixture".into(),
                            email: "fixture@example.test".into(),
                        },
                    )
                    .await
                    .unwrap(),
                );
            }
            identities::set_default(&pool, ids[0], 1, true)
                .await
                .unwrap();
            for id in [outer, inner] {
                save_scan(
                    &pool,
                    &directory(&pool, id).await.unwrap(),
                    vec![DiscoveredRepository {
                        path: repo_path.to_str().unwrap().into(),
                        kind: "ordinary",
                        git_dir: repo_path.join(".git").to_str().unwrap().into(),
                        common_dir: repo_path.join(".git").to_str().unwrap().into(),
                    }],
                    false,
                )
                .await
                .unwrap();
            }
            let repo_id = contents(&pool, outer).await.unwrap().repositories[0].id;
            assert_eq!(
                contents(&pool, inner).await.unwrap().repositories[0].id,
                repo_id
            );
            let repo = repository(&pool, repo_id).await.unwrap();
            assert_eq!(
                expected(&pool, &repo).await.unwrap().unwrap().source,
                "default"
            );
            for (id, identity) in [(outer, ids[1]), (inner, ids[2])] {
                assign(
                    &pool,
                    false,
                    id,
                    directory(&pool, id).await.unwrap().revision,
                    Some(identity),
                )
                .await
                .unwrap();
            }
            let expected_rule = expected(&pool, &repo).await.unwrap().unwrap();
            assert_eq!(expected_rule.label, "trabajo");
            assert_eq!(expected_rule.source, "directory");
            assign(&pool, true, repo.id, repo.revision, Some(ids[3]))
                .await
                .unwrap();
            assert_eq!(
                expected(&pool, &repository(&pool, repo.id).await.unwrap())
                    .await
                    .unwrap()
                    .unwrap()
                    .label,
                "excepción"
            );
            assert_eq!(
                identities::delete(&pool, ids[3], 1).await,
                Err(crate::identity::IdentityError::InUse)
            );
            let snapshot = directory(&pool, inner).await.unwrap();
            save_scan(&pool, &snapshot, vec![], true).await.unwrap();
            assert!(contents(&pool, inner).await.unwrap().repositories[0].present);
            assert_eq!(
                save_scan(&pool, &snapshot, vec![], false).await,
                Err(InventoryError::Conflict)
            );
            save_scan(
                &pool,
                &directory(&pool, inner).await.unwrap(),
                vec![],
                false,
            )
            .await
            .unwrap();
            assert!(!contents(&pool, inner).await.unwrap().repositories[0].present);
            assert!(contents(&pool, outer).await.unwrap().repositories[0].present);
            assign(
                &pool,
                true,
                repo.id,
                repository(&pool, repo.id).await.unwrap().revision,
                None,
            )
            .await
            .unwrap();
            assert!(contents(&pool, inner)
                .await
                .unwrap()
                .repositories
                .is_empty());
            pool.close().await;
        });
    }
}
