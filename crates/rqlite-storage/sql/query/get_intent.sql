SELECT
    intents.intent
FROM
    intents
WHERE
    intents.content_hash = ?;