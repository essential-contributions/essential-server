SELECT
    solution
FROM
    solutions_pool
    JOIN solutions ON solutions_pool.content_hash = solutions.content_hash