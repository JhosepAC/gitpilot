CREATE TABLE identities (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    label TEXT NOT NULL CHECK (length(label) BETWEEN 1 AND 80),
    git_name TEXT NOT NULL CHECK (length(git_name) BETWEEN 1 AND 120),
    email TEXT NOT NULL CHECK (length(email) BETWEEN 3 AND 254),
    is_default INTEGER NOT NULL DEFAULT 0 CHECK (is_default IN (0, 1)),
    revision INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE UNIQUE INDEX identities_single_default ON identities (is_default) WHERE is_default = 1;
