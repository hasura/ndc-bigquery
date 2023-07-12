use query_engine::phases::translation;
use std::fs;

/// Run a query against the server, get the result, and compare against the snapshot.
pub fn test_translation(testname: &str) -> Result<String, translation::Error> {
    let tables = serde_json::from_str(
        fs::read_to_string(format!("tests/goldenfiles/{}/tables.json", testname))
            .unwrap()
            .as_str(),
    )
    .unwrap();
    let request = serde_json::from_str(
        fs::read_to_string(format!("tests/goldenfiles/{}/request.json", testname))
            .unwrap()
            .as_str(),
    )
    .unwrap();

    let plan = translation::translate(&tables, request)?;

    Ok(plan.query().sql)
}
