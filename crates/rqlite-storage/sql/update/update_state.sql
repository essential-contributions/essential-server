INSERT INTO intent_state (set_id, key, value)
SELECT id, ?, ?
FROM intent_sets
WHERE content_hash = ?
ON CONFLICT (set_id, key) DO UPDATE SET value = EXCLUDED.value;
