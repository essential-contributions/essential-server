use super::*;
use crate::encode;
use serde_json::Number;
use test_utils::{empty::Empty, sign_with_random_keypair};

#[test]
fn test_valid_query() {
    let Signed { data, signature } = sign_with_random_keypair(Solution::empty());
    let r = map_solution_to_block(
        Default::default(),
        &[
            Value::Number(Number::from(1)),
            Value::String(encode(&data)),
            Value::String(encode(&signature)),
            Value::Number(Number::from(2)),
            Value::Number(Number::from(3)),
        ],
    )
    .unwrap();
    assert_eq!(r.len(), 1);
    assert_eq!(r[&1].batch.solutions.len(), 1);
    assert_eq!(
        r[&1].batch.solutions[0],
        Signed {
            data: data.clone(),
            signature: signature.clone()
        }
    );
    assert_eq!(r[&1].number, 0);
    assert_eq!(r[&1].timestamp, Duration::new(2, 3));

    let Signed {
        data: data2,
        signature: signature2,
    } = sign_with_random_keypair(Solution::empty());
    let r = map_solution_to_block(
        r,
        &[
            Value::Number(Number::from(1)),
            Value::String(encode(&data2)),
            Value::String(encode(&signature2)),
            Value::Number(Number::from(9)),
            Value::Number(Number::from(10)),
        ],
    )
    .unwrap();
    assert_eq!(r.len(), 1);
    assert_eq!(r[&1].batch.solutions.len(), 2);
    assert_eq!(
        r[&1].batch.solutions[0],
        Signed {
            data: data.clone(),
            signature: signature.clone()
        }
    );
    assert_eq!(
        r[&1].batch.solutions[1],
        Signed {
            data: data2.clone(),
            signature: signature2.clone()
        }
    );
    assert_eq!(r[&1].number, 0);
    assert_eq!(r[&1].timestamp, Duration::new(2, 3));

    let Signed {
        data: data3,
        signature: signature3,
    } = sign_with_random_keypair(Solution::empty());
    let r = map_solution_to_block(
        r,
        &[
            Value::Number(Number::from(2)),
            Value::String(encode(&data3)),
            Value::String(encode(&signature3)),
            Value::Number(Number::from(11)),
            Value::Number(Number::from(12)),
        ],
    )
    .unwrap();
    assert_eq!(r.len(), 2);
    assert_eq!(r[&1].batch.solutions.len(), 2);
    assert_eq!(r[&1].batch.solutions[0], Signed { data, signature });
    assert_eq!(
        r[&1].batch.solutions[1],
        Signed {
            data: data2,
            signature: signature2
        }
    );
    assert_eq!(
        r[&2].batch.solutions[0],
        Signed {
            data: data3,
            signature: signature3
        }
    );
    assert_eq!(r[&1].number, 0);
    assert_eq!(r[&1].timestamp, Duration::new(2, 3));
    assert_eq!(r[&2].number, 1);
    assert_eq!(r[&2].timestamp, Duration::new(11, 12));
}

#[test]
fn test_block_id_zero() {
    let Signed { data, signature } = sign_with_random_keypair(Solution::empty());
    map_solution_to_block(
        Default::default(),
        &[
            Value::Number(Number::from(0)),
            Value::String(encode(&data)),
            Value::String(encode(&signature)),
            Value::Number(Number::from(1)),
            Value::Number(Number::from(1)),
        ],
    )
    .unwrap_err();
}

#[test]
fn test_invalid_data() {
    let invalid = "xxxxxxxxxx".to_string();
    let Signed { data, signature } = sign_with_random_keypair(Solution::empty());

    map_solution_to_block(
        Default::default(),
        &[
            Value::Bool(true),
            Value::String(encode(&data)),
            Value::String(encode(&signature)),
            Value::Number(Number::from(2)),
            Value::Number(Number::from(3)),
        ],
    )
    .unwrap_err();

    map_solution_to_block(
        Default::default(),
        &[
            Value::Number(Number::from(1)),
            Value::Bool(true),
            Value::String(encode(&signature)),
            Value::Number(Number::from(2)),
            Value::Number(Number::from(3)),
        ],
    )
    .unwrap_err();

    map_solution_to_block(
        Default::default(),
        &[
            Value::Number(Number::from(1)),
            Value::String(encode(&data)),
            Value::Bool(true),
            Value::Number(Number::from(2)),
            Value::Number(Number::from(3)),
        ],
    )
    .unwrap_err();

    map_solution_to_block(
        Default::default(),
        &[
            Value::Number(Number::from(1)),
            Value::String(encode(&data)),
            Value::String(encode(&signature)),
            Value::Bool(true),
            Value::Number(Number::from(3)),
        ],
    )
    .unwrap_err();

    map_solution_to_block(
        Default::default(),
        &[
            Value::Number(Number::from(1)),
            Value::String(encode(&data)),
            Value::String(encode(&signature)),
            Value::Number(Number::from(2)),
            Value::Bool(true),
        ],
    )
    .unwrap_err();

    map_solution_to_block(
        Default::default(),
        &[
            Value::Number(Number::from(1)),
            Value::String(invalid.clone()),
            Value::String(encode(&signature)),
            Value::Number(Number::from(2)),
            Value::Number(Number::from(3)),
        ],
    )
    .unwrap_err();

    map_solution_to_block(
        Default::default(),
        &[
            Value::Number(Number::from(1)),
            Value::String(encode(&data)),
            Value::String(invalid.clone()),
            Value::Number(Number::from(2)),
            Value::Number(Number::from(3)),
        ],
    )
    .unwrap_err();

    map_solution_to_block(
        Default::default(),
        &[
            Value::Number(Number::from_f64(1.0).unwrap()),
            Value::String(encode(&data)),
            Value::String(encode(&signature)),
            Value::Number(Number::from(2)),
            Value::Number(Number::from(3)),
        ],
    )
    .unwrap_err();

    map_solution_to_block(
        Default::default(),
        &[
            Value::Number(Number::from(1)),
            Value::String(encode(&data)),
            Value::String(encode(&signature)),
            Value::Number(Number::from_f64(1.0).unwrap()),
            Value::Number(Number::from(3)),
        ],
    )
    .unwrap_err();

    map_solution_to_block(
        Default::default(),
        &[
            Value::Number(Number::from(1)),
            Value::String(encode(&data)),
            Value::String(encode(&signature)),
            Value::Number(Number::from(2)),
            Value::Number(Number::from_f64(1.0).unwrap()),
        ],
    )
    .unwrap_err();
}

#[test]
fn test_wrong_num_columns() {
    let Signed { data, signature } = sign_with_random_keypair(Solution::empty());
    map_solution_to_block(
        Default::default(),
        &[
            Value::Number(Number::from(1)),
            Value::String(encode(&data)),
            Value::String(encode(&signature)),
            Value::Number(Number::from(2)),
        ],
    )
    .unwrap_err();

    map_solution_to_block(
        Default::default(),
        &[
            Value::Number(Number::from(1)),
            Value::String(encode(&data)),
            Value::String(encode(&signature)),
            Value::Number(Number::from(2)),
            Value::Number(Number::from(3)),
            Value::Number(Number::from(3)),
        ],
    )
    .unwrap_err();
}
