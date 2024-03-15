DELETE FROM
    eoa_state
WHERE
    eoa_id = (
        SELECT
            eoa.id
        FROM
            eoa
        WHERE
            eoa.public_key = ?
    )
    AND KEY = ?;