SELECT
    solutions.solution,
    batch_id AS block_number,
    NULL AS reason
FROM
    solved
JOIN
    solutions ON solved.content_hash = solutions.content_hash
WHERE
    solved.content_hash = ?
UNION
ALL
SELECT
    solutions.solution,
    NULL AS block_number,
    reason
FROM
    failed_solutions
JOIN
    solutions ON failed_solutions.content_hash = solutions.content_hash
WHERE
    failed_solutions.content_hash = ?;