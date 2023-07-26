mod common;

#[test]
fn it_converts_select_with_limit() {
    let result = common::test_translation("select_with_limit").unwrap();
    insta::assert_snapshot!(result);
}

#[test]
fn it_select_where_string() {
    let result = common::test_translation("select_where_string").unwrap();
    insta::assert_snapshot!(result);
}

#[test]
fn it_aggregate_count_albums() {
    let result = common::test_translation("aggregate_count_albums").unwrap();
    insta::assert_snapshot!(result);
}

#[test]
fn it_aggregate_distinct_albums() {
    let result = common::test_translation("aggregate_distinct_albums").unwrap();
    insta::assert_snapshot!(result);
}

#[test]
fn it_aggregate_function_albums() {
    let result = common::test_translation("aggregate_function_albums").unwrap();
    insta::assert_snapshot!(result);
}
