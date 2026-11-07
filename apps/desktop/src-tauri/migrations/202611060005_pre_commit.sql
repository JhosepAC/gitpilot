ALTER TABLE config_operations ADD COLUMN is_hook INTEGER NOT NULL DEFAULT 0 CHECK (is_hook IN (0, 1));
DROP INDEX config_live_by_context;
CREATE UNIQUE INDEX config_live_by_context ON config_operations (COALESCE(repository_id, 0), is_hook)
    WHERE state IN ('preparing', 'applied', 'restoring');
