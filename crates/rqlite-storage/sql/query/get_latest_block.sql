SELECT
    batch_id,
    solutions.solution,
    batch.created_at_seconds,
    batch.created_at_nanos
FROM
    solved
    JOIN solutions ON solved.content_hash = solutions.content_hash
    JOIN batch ON solved.batch_id = batch.id
WHERE
    batch_id IN (
        SELECT
            MAX(id)
        FROM
            batch
    );