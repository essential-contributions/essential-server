SELECT
    solved.batch_id,
    solutions.solution,
    batch.created_at_seconds,
    batch.created_at_nanos
FROM
    solved
    JOIN solutions ON solved.content_hash = solutions.content_hash
    JOIN batch ON solved.batch_id = batch.id
WHERE
    batch_id - 1 >= :page_size * :page_number
    AND batch_id - 1 < :page_size * :page_number + :page_size
ORDER BY
    batch_id ASC;