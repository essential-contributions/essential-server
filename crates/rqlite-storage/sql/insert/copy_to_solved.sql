INSERT
    OR IGNORE INTO solved (content_hash, batch_id)
SELECT
    content_hash,
    COALESCE(
        (
            SELECT
                MAX(id)
            FROM
                batch
        ),
        0
    )
FROM
    solutions_pool
WHERE
    content_hash = ?;