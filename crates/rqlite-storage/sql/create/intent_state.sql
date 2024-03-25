CREATE TABLE IF NOT EXISTS intent_state (
    id INTEGER PRIMARY KEY,
    set_id INTEGER NOT NULL,
    key BLOB NOT NULL,
    value INTEGER NOT NULL,
    FOREIGN KEY (set_id) REFERENCES intent_sets (id),
    UNIQUE(set_id, key)
);