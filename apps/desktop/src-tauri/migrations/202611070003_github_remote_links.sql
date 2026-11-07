CREATE TABLE github_remote_links (
    local_repository_id INTEGER NOT NULL REFERENCES repositories(id) ON DELETE CASCADE,
    remote TEXT NOT NULL,
    client_id TEXT NOT NULL,
    github_id INTEGER NOT NULL,
    installation_id INTEGER NOT NULL,
    repository_id INTEGER NOT NULL,
    fingerprint TEXT NOT NULL,
    version TEXT NOT NULL,
    PRIMARY KEY (local_repository_id, remote),
    FOREIGN KEY (client_id, github_id) REFERENCES github_accounts(client_id, github_id) ON DELETE CASCADE
);

CREATE INDEX github_remote_links_account ON github_remote_links(client_id, github_id);
