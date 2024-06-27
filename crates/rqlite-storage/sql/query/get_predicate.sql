SELECT
    contracts.predicate
FROM
    contract
    JOIN contract_pairing ON contracts.id = contract_pairing.predicate_id
    JOIN contracts ON contracts.id = contract_pairing.contract_id
WHERE
    contracts.content_hash = ?
    AND contracts.content_hash = ?;