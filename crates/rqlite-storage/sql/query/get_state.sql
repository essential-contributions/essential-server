SELECT intent_state.value
FROM intent_state
JOIN intent_sets ON intent_state.set_id = intent_sets.id
WHERE intent_sets.content_hash = ? AND intent_state.key = ?;