CREATE TABLE github_credentials (
    reference TEXT PRIMARY KEY,
    state TEXT NOT NULL CHECK (state IN ('pending', 'active', 'retiring'))
);

CREATE TABLE github_accounts (
    slot INTEGER PRIMARY KEY CHECK (slot = 1),
    github_id INTEGER NOT NULL CHECK (github_id > 0),
    client_id TEXT NOT NULL,
    login TEXT NOT NULL,
    expires_at INTEGER NOT NULL,
    credential_ref TEXT NOT NULL UNIQUE REFERENCES github_credentials(reference),
    verified_at INTEGER NOT NULL
);
