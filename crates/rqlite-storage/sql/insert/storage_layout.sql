INSERT OR IGNORE INTO
    storage_layout (layout, set_id)
SELECT
    ?,
    id
FROM
    intent_sets
WHERE
    content_hash = ?;