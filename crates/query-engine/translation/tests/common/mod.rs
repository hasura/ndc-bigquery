use std::fs;

use query_engine_sql::sql;
use query_engine_translation::translation;
use std::path::PathBuf;

/// Run a query against the server, get the result, and compare against the snapshot.
pub async fn test_translation(testname: &str) -> anyhow::Result<String> {
    let directory = PathBuf::from("tests/goldenfiles").join(testname);

    let parsed_configuration = ndc_bigquery_configuration::parse_configuration(&directory).await?;
    let configuration = ndc_bigquery_configuration::make_runtime_configuration(
        parsed_configuration,
        ndc_bigquery_configuration::environment::FixedEnvironment::from([
            (
                "HASURA_BIGQUERY_SERVICE_KEY".into(),
                "the translation tests do not rely on a database connection".into(),
            ),
            (
                "HASURA_BIGQUERY_PROJECT_ID".into(),
                "the translation tests do not rely on a database connection".into(),
            ),
            (
                "HASURA_BIGQUERY_DATASET_ID".into(),
                "the translation tests do not rely on a database connection".into(),
            ),
        ]),
    )?;
    let metadata = configuration.metadata;

    let request =
        serde_json::from_str(&fs::read_to_string(directory.join("request.json")).unwrap()).unwrap();

    let plan = translation::query::translate(&metadata, request)?;
    let query = plan.query.query_sql();
    let params: Vec<(usize, &sql::string::Param)> = query
        .params
        .iter()
        .enumerate()
        .map(|(i, p)| (i + 1, p))
        .collect();

    let pretty = sqlformat::format(
        &query.sql,
        &sqlformat::QueryParams::None,
        sqlformat::FormatOptions::default(),
    );

    Ok(format!("{}\n\n{:?}", pretty, params))
}
