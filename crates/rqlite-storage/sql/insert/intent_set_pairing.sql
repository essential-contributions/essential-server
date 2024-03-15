INSERT OR IGNORE INTO
    intent_set_pairing (set_id, intent_id)
VALUES
    (
        (
            SELECT
                id
            FROM
                intent_sets
            WHERE
                content_hash = ?
            LIMIT
                1
        ), (
            SELECT
                id
            FROM
                intents
            WHERE
                content_hash = ?
            LIMIT
                1
        )
    );