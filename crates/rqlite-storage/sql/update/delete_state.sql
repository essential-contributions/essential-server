DELETE FROM
    contract_state
WHERE
    contract_id = (
        SELECT
            contracts.id
        FROM
            contracts
        WHERE
            contracts.content_hash = ?
    )
    AND KEY = ?;