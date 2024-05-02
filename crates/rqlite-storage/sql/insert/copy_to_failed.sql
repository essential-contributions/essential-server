INSERT
    OR IGNORE INTO failed_solutions (content_hash, solution, signature, reason, created_at_seconds)
SELECT
    content_hash,
    solution,
    signature,
    ?,
    -- reason
    ? -- created_at_seconds
FROM
    solutions_pool
WHERE
    content_hash = ?;