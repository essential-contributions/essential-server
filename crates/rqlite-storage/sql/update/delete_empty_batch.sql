WITH unsolved_latest_batch AS (
    SELECT b.id
    FROM (
        SELECT MAX(id) AS id
        FROM batch
    ) lb
    LEFT JOIN solved s ON lb.id = s.batch_id
    JOIN batch b ON lb.id = b.id
    WHERE s.id IS NULL
)

DELETE FROM batch
WHERE id IN (SELECT id FROM unsolved_latest_batch);