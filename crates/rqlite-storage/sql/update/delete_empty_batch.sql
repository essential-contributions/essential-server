DELETE FROM
    batch
WHERE
    id = (
        SELECT
            b.id
        FROM
            (SELECT MAX(id) as id FROM batch) b
            LEFT JOIN solved s ON b.id = s.batch_id
        WHERE
            s.id IS NULL
        LIMIT
            1
    );