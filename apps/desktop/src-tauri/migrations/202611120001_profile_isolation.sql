ALTER TABLE identities ADD COLUMN signing_required BOOLEAN NOT NULL DEFAULT 1 CHECK (signing_required IN (0, 1));
UPDATE identities SET signing_required = 0 WHERE signing_key IS NULL;

-- Applied operations form a restorable stack. Only a write in progress is unique.
DROP INDEX config_live_by_context;
DROP INDEX config_live_by_target;
CREATE UNIQUE INDEX config_pending_by_target ON config_operations (target)
    WHERE state IN ('preparing', 'restoring');
