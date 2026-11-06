CREATE TABLE directories (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    path TEXT NOT NULL UNIQUE,
    revision INTEGER NOT NULL DEFAULT 1,
    scan_partial INTEGER CHECK (scan_partial IN (0, 1))
);

CREATE TABLE directory_exclusions (
    directory_id INTEGER NOT NULL REFERENCES directories(id) ON DELETE CASCADE,
    path TEXT NOT NULL,
    PRIMARY KEY (directory_id, path)
);

CREATE TABLE repositories (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    path TEXT NOT NULL UNIQUE,
    kind TEXT NOT NULL CHECK (kind IN ('ordinary', 'worktree', 'separate', 'bare')),
    git_dir TEXT NOT NULL,
    common_dir TEXT NOT NULL,
    revision INTEGER NOT NULL DEFAULT 1
);

CREATE TABLE directory_repositories (
    directory_id INTEGER NOT NULL REFERENCES directories(id) ON DELETE CASCADE,
    repository_id INTEGER NOT NULL REFERENCES repositories(id) ON DELETE CASCADE,
    present INTEGER NOT NULL DEFAULT 1 CHECK (present IN (0, 1)),
    PRIMARY KEY (directory_id, repository_id)
);

CREATE INDEX directory_repositories_repository ON directory_repositories(repository_id);

CREATE TABLE identity_rules (
    identity_id INTEGER NOT NULL REFERENCES identities(id) ON DELETE RESTRICT,
    directory_id INTEGER UNIQUE REFERENCES directories(id) ON DELETE CASCADE,
    repository_id INTEGER UNIQUE REFERENCES repositories(id) ON DELETE CASCADE,
    CHECK ((directory_id IS NULL) <> (repository_id IS NULL))
);

CREATE INDEX identity_rules_identity ON identity_rules(identity_id);
