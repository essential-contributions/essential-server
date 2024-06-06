SELECT
    solution
FROM
    solutions_pool
    JOIN solutions ON solutions_pool.content_hash = solutions.content_hash
ORDER BY
    solutions_pool.id
LIMIT
    :page_size * :page_number, :page_size;