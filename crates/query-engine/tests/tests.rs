mod common;

#[test]
fn it_converts_select_with_limit() {
    let result = common::test_translation("select_with_limit").unwrap();
    insta::assert_snapshot!(result);
}
