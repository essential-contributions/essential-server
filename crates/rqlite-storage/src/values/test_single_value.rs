use super::*;

#[test]
fn test_single_value_with_one_row_one_column() {
    let value = Value::Bool(true);
    let columns = Columns {
        columns: vec![value.clone()],
    };
    let rows = Rows {
        rows: vec![columns],
    };
    let queries = QueryValues {
        queries: vec![Some(rows)],
    };

    assert_eq!(single_value(&queries), Some(&value));
}

#[test]
fn test_single_value_with_multiple_rows() {
    let value = Value::Bool(true);
    let columns = Columns {
        columns: vec![value.clone()],
    };
    let rows = Rows {
        rows: vec![columns.clone(), columns.clone()],
    };
    let queries = QueryValues {
        queries: vec![Some(rows)],
    };

    assert_eq!(single_value(&queries), None);
}

#[test]
fn test_single_value_with_multiple_columns() {
    let value = Value::Bool(true);
    let columns = Columns {
        columns: vec![value.clone(), value.clone()],
    };
    let rows = Rows {
        rows: vec![columns],
    };
    let queries = QueryValues {
        queries: vec![Some(rows)],
    };

    assert_eq!(single_value(&queries), None);
}

#[test]
fn test_single_value_with_no_rows() {
    let queries = QueryValues {
        queries: vec![None],
    };

    assert_eq!(single_value(&queries), None);
}
