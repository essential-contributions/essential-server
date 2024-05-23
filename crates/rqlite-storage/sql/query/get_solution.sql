SELECT
    solution,
    batch_id AS block_number,
    NULL AS reason
FROM
    solved
WHERE
    content_hash = ?
UNION
ALL
SELECT
    solution,
    NULL AS block_number,
    reason
FROM
    failed_solutions
WHERE
    content_hash = ?;