CREATE TABLE IF NOT EXISTS contents (
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    source TEXT,
    created_at TEXT NOT NULL,
    summary TEXT,
    source_master_id TEXT,
    is_translating INTEGER NOT NULL DEFAULT 0,
    source_url TEXT
);

CREATE TABLE IF NOT EXISTS sentences (
    id TEXT PRIMARY KEY,
    content_id TEXT NOT NULL,
    sentence_index INTEGER NOT NULL,
    english_text TEXT NOT NULL,
    japanese_text TEXT,
    created_at TEXT NOT NULL,
    text_completed INTEGER NOT NULL DEFAULT 0,
    speech_completed INTEGER NOT NULL DEFAULT 0,
    FOREIGN KEY (content_id) REFERENCES contents(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS source_masters (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    translate_prompt TEXT,
    created_at TEXT NOT NULL
);
