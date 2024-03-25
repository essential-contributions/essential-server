INSERT INTO eoa_state (eoa_id, key, value)
SELECT id, ?, ?
FROM eoa
WHERE public_key = ?
ON CONFLICT (eoa_id, key) DO UPDATE SET value = EXCLUDED.value;
