SELECT
    solved.batch_id,
    solved.solution,
    batch.created_at_seconds,
    batch.created_at_nanos
FROM
    solved
    JOIN batch ON solved.batch_id = batch.id
WHERE
    batch_id IN (
        SELECT
            id
        FROM
            batch
        WHERE
            (
                created_at_seconds > :start_seconds
                OR (
                    created_at_seconds = :start_seconds
                    AND created_at_nanos >= :start_nanos
                )
            )
            AND (
                created_at_seconds < :end_seconds
                OR (
                    created_at_seconds = :end_seconds
                    AND created_at_nanos <= :end_nanos
                )
            )
        ORDER BY
            id ASC
        LIMIT
            :page_size OFFSET :page_number * :page_size
    )
ORDER BY
    batch_id ASC;