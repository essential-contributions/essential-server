-- Step 1: Find the set_id range for the desired page
WITH ranked_sets AS (
    SELECT
        id,
        ROW_NUMBER() OVER (
            ORDER BY
                id
        ) AS rownum
    FROM
        (
            SELECT
                DISTINCT set_id AS id
            FROM
                intent_set_pairing
        )
),
set_range AS (
    SELECT
        MIN(id) AS min_id,
        MAX(id) AS max_id
    FROM
        ranked_sets
    WHERE
        rownum - 1 >= :page_size * :page_number
        AND rownum - 1 < :page_size * :page_number + :page_size
) -- Step 2: Retrieve intents for the sets in the range
SELECT
    isp.set_id,
    i.intent
FROM
    intent_set_pairing isp
    JOIN set_range sr ON isp.set_id BETWEEN sr.min_id
    AND sr.max_id
    JOIN intents i ON isp.intent_id = i.id
ORDER BY
    isp.set_id,
    isp.id;