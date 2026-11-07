CREATE TABLE portable_activity (
    id INTEGER PRIMARY KEY,
    action TEXT NOT NULL CHECK (action IN ('imported', 'exported')),
    profile_count INTEGER NOT NULL CHECK (profile_count >= 0),
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);
