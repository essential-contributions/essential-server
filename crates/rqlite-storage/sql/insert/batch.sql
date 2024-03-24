INSERT
    OR IGNORE INTO batch (content_hash, created_at_seconds, created_at_nanos)
VALUES
    (?, ?, ?);