SELECT
    batch_id AS block_number,
    NULL AS reason,
    created_at_seconds,
    created_at_nanos
FROM
    solved
JOIN
    batch ON solved.batch_id = batch.id
WHERE
    solved.content_hash = ?
UNION
ALL
SELECT
    NULL AS block_number,
    reason,
    created_at_seconds,
    created_at_nanos
FROM
    failed_solutions
WHERE
    failed_solutions.content_hash = ?
ORDER BY
    created_at_seconds ASC,
    created_at_nanos ASC;