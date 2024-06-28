SELECT
    id,
    salt
FROM
    contracts
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
    id
LIMIT
    :page_size OFFSET :page_size * :page_number;