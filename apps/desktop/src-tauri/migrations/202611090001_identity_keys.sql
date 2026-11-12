ALTER TABLE identities ADD COLUMN signing_key TEXT CHECK (signing_key IS NULL OR length(signing_key) <= 500);
ALTER TABLE identities ADD COLUMN signing_format TEXT CHECK (signing_format IS NULL OR signing_format IN ('ssh', 'openpgp'));
ALTER TABLE identities ADD COLUMN ssh_key TEXT CHECK (ssh_key IS NULL OR length(ssh_key) <= 500);
