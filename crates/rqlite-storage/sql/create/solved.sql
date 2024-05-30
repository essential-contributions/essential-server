CREATE TABLE IF NOT EXISTS solved (
    id INTEGER PRIMARY KEY,
    content_hash BLOB NOT NULL,
    batch_id INTEGER NOT NULL,
    FOREIGN KEY (batch_id) REFERENCES batch (id),
    FOREIGN KEY (content_hash) REFERENCES solutions (content_hash)
);