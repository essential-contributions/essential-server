SELECT
    intent_set_pairing.set_id,
    intents.intent
FROM
    intents
    JOIN intent_set_pairing ON intents.id = intent_set_pairing.intent_id
WHERE
    intent_set_pairing.set_id IN (
        SELECT
            id
        FROM
            intent_sets
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
        LIMIT
            :page_size OFFSET :page_size * :page_number
    )
ORDER BY
    intent_set_pairing.set_id,
    intent_set_pairing.id;