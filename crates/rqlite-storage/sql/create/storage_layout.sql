CREATE TABLE IF NOT EXISTS storage_layout (
    id INTEGER PRIMARY KEY,
    layout BLOB NOT NULL UNIQUE,
    set_id INTEGER NOT NULL,
    FOREIGN KEY (set_id) REFERENCES intent_sets (id)
);
