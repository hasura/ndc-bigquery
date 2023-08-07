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

#[test]
fn it_simple_array_relationship() {
    let result = common::test_translation("simple_array_relationship").unwrap();
    insta::assert_snapshot!(result);
}

#[test]
fn it_simple_object_relationship() {
    let result = common::test_translation("simple_object_relationship").unwrap();
    insta::assert_snapshot!(result);
}

#[test]
fn nested_array_relationships() {
    let result = common::test_translation("nested_array_relationships").unwrap();
    insta::assert_snapshot!(result);
}

#[test]
fn nested_aggregates() {
    let result = common::test_translation("nested_aggregates").unwrap();
    insta::assert_snapshot!(result);
}

#[test]
fn dup_array_relationship() {
    let result = common::test_translation("dup_array_relationship").unwrap();
    insta::assert_snapshot!(result);
}

#[test]
fn sorting_by_relationship_column() {
    let result = common::test_translation("sorting_by_relationship_column").unwrap();
    insta::assert_snapshot!(result);
}

#[test]
fn sorting_by_nested_relationship_column() {
    let result = common::test_translation("sorting_by_nested_relationship_column").unwrap();
    insta::assert_snapshot!(result);
}
