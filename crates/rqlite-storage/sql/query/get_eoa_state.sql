SELECT eoa_state.value
FROM eoa_state
JOIN eoa ON eoa_state.eoa_id = eoa.id
WHERE eoa.public_key = ? AND eoa_state.key = ?;