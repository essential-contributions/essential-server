use super::*;

#[test]
fn test_empty_query() {
    let queries = QueryValues {
        queries: vec![None],
    };

    assert!(list_winning_blocks(queries).unwrap().is_empty());
}

#[test]
fn test_invalid_query() {
    let queries = QueryValues { queries: vec![] };

    list_winning_blocks(queries).unwrap_err();

    let queries = QueryValues {
        queries: vec![None, None],
    };

    list_winning_blocks(queries).unwrap_err();
}

#[test]
fn test_wrong_num_rows() {
    let queries = QueryValues {
        queries: vec![Some(Rows { rows: vec![] })],
    };
    list_winning_blocks(queries).unwrap_err();
}
