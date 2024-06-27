INSERT
    OR IGNORE INTO contracts (
        content_hash,
        signature,
        created_at_seconds,
        created_at_nanos
    )
VALUES
    (?, ?, ?, ?)