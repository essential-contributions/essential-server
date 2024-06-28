SELECT contract_state.value
FROM contract_state
JOIN contracts ON contract_state.contract_id = contracts.id
WHERE contracts.content_hash = ? AND contract_state.key = ?;