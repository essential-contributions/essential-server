SELECT
    intents.intent
FROM
    intents
    JOIN intent_set_pairing ON intents.id = intent_set_pairing.intent_id
    JOIN intent_sets ON intent_sets.id = intent_set_pairing.set_id
WHERE
    intent_sets.content_hash = ?
    AND intents.content_hash = ?;