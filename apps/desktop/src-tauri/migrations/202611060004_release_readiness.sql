ALTER TABLE config_operations ADD COLUMN repository_id INTEGER;
ALTER TABLE config_operations ADD COLUMN updated_at TEXT;
DROP INDEX config_single_live_operation;
CREATE UNIQUE INDEX config_live_by_context ON config_operations (COALESCE(repository_id, 0))
    WHERE state IN ('preparing', 'applied', 'restoring');
CREATE UNIQUE INDEX config_live_by_target ON config_operations (target)
    WHERE state IN ('preparing', 'applied', 'restoring');

CREATE TABLE conflict_decisions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    repository_id INTEGER NOT NULL,
    path TEXT NOT NULL,
    fingerprint TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE (repository_id, fingerprint)
);
