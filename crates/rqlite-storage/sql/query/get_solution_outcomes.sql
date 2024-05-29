SELECT
    batch_id AS block_number,
    NULL AS reason
FROM
    solved
WHERE
    solved.content_hash = ?
UNION
ALL
SELECT
    NULL AS block_number,
    reason
FROM
    failed_solutions
WHERE
    failed_solutions.content_hash = ?;