INSERT
    OR IGNORE INTO failed_solutions (content_hash, solution, signature, reason)
SELECT
    content_hash,
    solution,
    signature,
    ?
FROM
    solutions_pool
WHERE
    content_hash = ?;