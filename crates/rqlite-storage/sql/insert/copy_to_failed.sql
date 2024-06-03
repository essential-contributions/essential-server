INSERT
    OR IGNORE INTO failed_solutions (content_hash, reason, created_at_seconds, created_at_nanos)
SELECT
    content_hash,
    ?,
    -- reason
    ?,
    -- created_at_seconds
    ? -- created_at_nanos
FROM
    solutions_pool
WHERE
    content_hash = ?;