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
