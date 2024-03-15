SELECT
    storage_layout.layout
FROM
    storage_layout
    JOIN intent_sets ON storage_layout.set_id = intent_sets.id
WHERE
    intent_sets.content_hash = ?