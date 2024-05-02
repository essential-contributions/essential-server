SELECT
    signature,
    solution,
    NULL AS reason
FROM
    solutions
UNION
ALL
SELECT
    signature,
    solution,
    reason
FROM
    failed_solutions;