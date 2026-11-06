CREATE TABLE config_operations (
    token TEXT PRIMARY KEY,
    target TEXT NOT NULL,
    parent_id TEXT NOT NULL,
    before_stamp TEXT,
    backup BLOB,
    candidate_hash TEXT NOT NULL,
    candidate_id TEXT,
    applied_stamp TEXT,
    recovery_id TEXT,
    state TEXT NOT NULL CHECK (state IN ('preparing', 'applied', 'restoring', 'restored', 'rolled_back')),
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CHECK ((before_stamp IS NULL) = (backup IS NULL))
);

CREATE UNIQUE INDEX config_single_live_operation ON config_operations ((1))
    WHERE state IN ('preparing', 'applied', 'restoring');
