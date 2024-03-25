DELETE FROM
    intent_state
WHERE
    set_id = (
        SELECT
            intent_sets.id
        FROM
            intent_sets
        WHERE
            intent_sets.content_hash = ?
    )
    AND KEY = ?;