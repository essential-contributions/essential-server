INSERT
    OR IGNORE INTO solved (content_hash, solution, signature, batch_id)
SELECT
    content_hash,
    solution,
    signature,
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