//! Handle the translation of literal values.

use super::error::Error;
use crate::phases::translation::sql;

/// Convert a JSON value into a SQL value.
pub fn translate_json_value(value: &serde_json::Value) -> Result<sql::ast::Value, Error> {
    match value {
        serde_json::Value::Number(num) => Ok(sql::ast::Value::Int4(
            num.as_i64().unwrap().try_into().unwrap(),
        )),
        serde_json::Value::Bool(b) => Ok(sql::ast::Value::Bool(*b)),
        serde_json::Value::String(s) => Ok(sql::ast::Value::String(s.to_string())),
        serde_json::Value::Null => Ok(sql::ast::Value::Null),
        serde_json::Value::Array(items) => {
            let inner_values: Vec<sql::ast::Value> = items
                .iter()
                .map(translate_json_value)
                .collect::<Result<Vec<sql::ast::Value>, Error>>(
            )?;
            Ok(sql::ast::Value::Array(inner_values))
        }
        serde_json::Value::Object(_) => Err(Error::NotSupported("object values".to_string())),
    }
}
