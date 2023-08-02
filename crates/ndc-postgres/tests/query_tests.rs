pub mod common;

mod basic {
    use super::common::run_query;

    #[tokio::test]
    async fn select_by_pk() {
        let result = run_query("select_by_pk").await;
        insta::assert_json_snapshot!(result);
    }

    #[tokio::test]
    async fn select_5() {
        let result = run_query("select_5").await;
        insta::assert_json_snapshot!(result);
    }

    #[tokio::test]
    async fn select_int_and_string() {
        let result = run_query("select_int_and_string").await;
        insta::assert_json_snapshot!(result);
    }
}

mod predicates {
    use super::common::run_query;

    #[tokio::test]
    async fn select_where_name_like() {
        let result = run_query("select_where_name_like").await;
        insta::assert_json_snapshot!(result);
    }

    #[tokio::test]
    async fn select_where_name_not_like() {
        let result = run_query("select_where_name_not_like").await;
        insta::assert_json_snapshot!(result);
    }

    #[tokio::test]
    async fn select_where_album_id_less_than() {
        let result = run_query("select_where_album_id_less_than").await;
        insta::assert_json_snapshot!(result);
    }

    #[tokio::test]
    async fn select_where_album_id_less_than_or_equal_to() {
        let result = run_query("select_where_album_id_less_than_or_equal_to").await;
        insta::assert_json_snapshot!(result);
    }

    #[tokio::test]
    async fn select_where_album_id_greater_than() {
        let result = run_query("select_where_album_id_greater_than").await;
        insta::assert_json_snapshot!(result);
    }

    #[tokio::test]
    async fn select_where_album_id_greater_than_or_equal_to() {
        let result = run_query("select_where_album_id_greater_than_or_equal_to").await;
        insta::assert_json_snapshot!(result);
    }

    #[tokio::test]
    async fn select_where_name_in() {
        let result = run_query("select_where_name_in").await;
        insta::assert_json_snapshot!(result);
    }

    #[tokio::test]
    async fn select_where_name_not_in() {
        let result = run_query("select_where_name_not_in").await;
        insta::assert_json_snapshot!(result);
    }

    #[tokio::test]
    async fn select_where_name_ilike() {
        let result = run_query("select_where_name_ilike").await;
        insta::assert_json_snapshot!(result);
    }

    #[tokio::test]
    async fn select_where_name_nilike() {
        let result = run_query("select_where_name_nilike").await;
        insta::assert_json_snapshot!(result);
    }

    #[tokio::test]
    async fn select_where_name_similar() {
        let result = run_query("select_where_name_similar").await;
        insta::assert_json_snapshot!(result);
    }

    #[tokio::test]
    async fn select_where_name_nsimilar() {
        let result = run_query("select_where_name_nsimilar").await;
        insta::assert_json_snapshot!(result);
    }

    #[tokio::test]
    async fn select_where_name_regex() {
        let result = run_query("select_where_name_regex").await;
        insta::assert_json_snapshot!(result);
    }

    #[tokio::test]
    async fn select_where_name_nregex() {
        let result = run_query("select_where_name_nregex").await;
        insta::assert_json_snapshot!(result);
    }

    #[tokio::test]
    async fn select_where_name_iregex() {
        let result = run_query("select_where_name_iregex").await;
        insta::assert_json_snapshot!(result);
    }

    #[tokio::test]
    async fn select_where_name_niregex() {
        let result = run_query("select_where_name_niregex").await;
        insta::assert_json_snapshot!(result);
    }

    #[tokio::test]
    async fn select_where_variable() {
        let result = run_query("select_where_variable").await;
        insta::assert_json_snapshot!(result);
    }

    #[tokio::test]
    async fn select_where_variable_int() {
        let result = run_query("select_where_variable_int").await;
        insta::assert_json_snapshot!(result);
    }
}

mod sorting {
    use super::common::run_query;

    #[tokio::test]
    async fn select_order_by_name() {
        let result = run_query("select_order_by_name").await;
        insta::assert_json_snapshot!(result);
    }

    #[tokio::test]
    async fn select_order_by_artist_name() {
        let result = run_query("select_order_by_artist_name").await;
        insta::assert_json_snapshot!(result);
    }

    #[tokio::test]
    async fn select_order_by_album_artist_name() {
        let result = run_query("select_order_by_album_artist_name").await;
        insta::assert_json_snapshot!(result);
    }
}

mod aggregation {
    use super::common::run_query;

    #[tokio::test]
    async fn aggregate_count_albums() {
        let result = run_query("aggregate_count_albums").await;
        insta::assert_json_snapshot!(result);
    }
}

mod relationships {
    use super::common::run_query;

    #[tokio::test]
    async fn select_album_object_relationship_to_artist() {
        let result = run_query("select_album_object_relationship_to_artist").await;
        insta::assert_json_snapshot!(result);
    }

    #[tokio::test]
    async fn select_artist_array_relationship_to_album() {
        let result = run_query("select_artist_array_relationship_to_album").await;
        insta::assert_json_snapshot!(result);
    }

    #[tokio::test]
    async fn nested_array_relationships() {
        let result = run_query("nested_array_relationships").await;
        insta::assert_json_snapshot!(result);
    }

    #[tokio::test]
    async fn nested_object_relationships() {
        let result = run_query("nested_object_relationships").await;
        insta::assert_json_snapshot!(result);
    }

    #[tokio::test]
    async fn dup_array_relationship() {
        let result = run_query("dup_array_relationship").await;
        insta::assert_json_snapshot!(result);
    }
}
