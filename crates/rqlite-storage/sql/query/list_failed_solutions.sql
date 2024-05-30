SELECT
    solution,
    reason
FROM
    failed_solutions
JOIN
    solutions ON failed_solutions.content_hash = solutions.content_hash
LIMIT
    :page_size * :page_number, :page_size;