-- Step 1: Retrieve the unique set_id values
WITH unique_set_ids AS (
    SELECT DISTINCT set_id
    FROM intent_set_pairing
    ORDER BY set_id
    LIMIT :page_size OFFSET :page_size * :page_number
)
-- Step 2: Retrieve intents for the sets in the range
SELECT
    isp.set_id,
    i.intent
FROM
    intent_set_pairing isp
    JOIN unique_set_ids usi ON isp.set_id = usi.set_id
    JOIN intents i ON isp.intent_id = i.id
ORDER BY
    isp.set_id,
    isp.id;