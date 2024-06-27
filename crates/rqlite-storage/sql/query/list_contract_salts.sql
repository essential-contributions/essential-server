SELECT
    id,
    salt
FROM
    contracts
ORDER BY
    id
LIMIT
    :page_size OFFSET :page_size * :page_number;