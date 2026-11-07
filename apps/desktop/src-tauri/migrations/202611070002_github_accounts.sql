ALTER TABLE github_credentials ADD COLUMN client_id TEXT;
ALTER TABLE github_credentials ADD COLUMN github_id INTEGER;

UPDATE github_credentials
SET client_id = (SELECT client_id FROM github_accounts WHERE credential_ref = reference),
    github_id = (SELECT github_id FROM github_accounts WHERE credential_ref = reference);

CREATE TABLE github_accounts_next (
    client_id TEXT NOT NULL,
    github_id INTEGER NOT NULL CHECK (github_id > 0),
    login TEXT NOT NULL,
    expires_at INTEGER NOT NULL,
    credential_ref TEXT NOT NULL UNIQUE REFERENCES github_credentials(reference),
    verified_at INTEGER NOT NULL,
    state TEXT NOT NULL DEFAULT 'active' CHECK (state IN ('active', 'refreshing', 'reauthorize', 'disconnecting')),
    refresh_ref TEXT UNIQUE REFERENCES github_credentials(reference),
    PRIMARY KEY (client_id, github_id)
);

INSERT INTO github_accounts_next (client_id, github_id, login, expires_at, credential_ref, verified_at)
SELECT client_id, github_id, login, expires_at, credential_ref, verified_at FROM github_accounts;
DROP TABLE github_accounts;
ALTER TABLE github_accounts_next RENAME TO github_accounts;

CREATE TABLE github_revocations (
    client_id TEXT NOT NULL,
    github_id INTEGER NOT NULL,
    login TEXT NOT NULL,
    PRIMARY KEY (client_id, github_id)
);
