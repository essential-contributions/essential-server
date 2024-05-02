DELETE FROM
    failed_solutions
WHERE
    created_at_seconds < ?;