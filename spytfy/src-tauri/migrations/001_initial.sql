CREATE TABLE IF NOT EXISTS settings (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL,
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS batches (
    id TEXT PRIMARY KEY,
    source_url TEXT NOT NULL,
    source_type TEXT NOT NULL CHECK (source_type IN ('track', 'album', 'playlist')),
    name TEXT NOT NULL,
    total_tracks INTEGER NOT NULL DEFAULT 0,
    state TEXT NOT NULL DEFAULT 'active',
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS jobs (
    id TEXT PRIMARY KEY,
    batch_id TEXT NOT NULL REFERENCES batches(id),
    spotify_id TEXT NOT NULL,
    title TEXT NOT NULL,
    artist TEXT NOT NULL,
    album TEXT NOT NULL,
    duration_ms INTEGER NOT NULL,
    state TEXT NOT NULL DEFAULT 'queued',
    yt_url TEXT,
    yt_score INTEGER,
    output_path TEXT,
    error TEXT,
    progress_pct REAL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX idx_jobs_batch ON jobs(batch_id);
CREATE INDEX idx_jobs_state ON jobs(state);
