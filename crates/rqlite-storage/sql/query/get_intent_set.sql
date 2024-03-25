SELECT
    intents.intent
FROM
    intent_set_pairing
    JOIN intent_sets ON intent_set_pairing.set_id = intent_sets.id
    JOIN intents ON intent_set_pairing.intent_id = intents.id
WHERE
    intent_sets.content_hash = ?;