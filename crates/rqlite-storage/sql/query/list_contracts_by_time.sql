SELECT
    contract_pairing.contract_id,
    contracts.predicate
FROM
    contracts
    JOIN contract_pairing ON contracts.id = contract_pairing.predicate_id
WHERE
    contract_pairing.contract_id IN (
        SELECT
            id
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
        LIMIT
            :page_size OFFSET :page_size * :page_number
    )
ORDER BY
    contract_pairing.contract_id,
    contract_pairing.id;