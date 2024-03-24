INSERT
    OR IGNORE INTO intent_sets (
        content_hash,
        signature,
        created_at_seconds,
        created_at_nanos
    )
VALUES
    (?, ?, ?, ?)