//! Handle the translation of literal values.

use super::error::Error;
use query_engine_metadata::metadata::database;
use query_engine_sql::sql;

/// Convert a JSON value into a SQL value.
pub fn translate_json_value(
    value: &serde_json::Value,
    scalar_type: database::ScalarType,
) -> Result<sql::ast::Value, Error> {
    match value {
        // numbers
        serde_json::Value::Number(num) => match scalar_type {
            // integers
            database::ScalarType::Smallint => Ok(sql::ast::Value::Int8(
                num.as_i64().unwrap().try_into().unwrap(),
            )),
            database::ScalarType::Integer => Ok(sql::ast::Value::Int8(
                num.as_i64().unwrap().try_into().unwrap(),
            )),
            database::ScalarType::Bigint => Ok(sql::ast::Value::Int8(
                num.as_i64().unwrap().try_into().unwrap(),
            )),

            // floats
            database::ScalarType::Real => Ok(sql::ast::Value::Float8(num.as_f64().unwrap())),
            database::ScalarType::DoublePrecision => {
                Ok(sql::ast::Value::Float8(num.as_f64().unwrap()))
            }
            database::ScalarType::Numeric => Ok(sql::ast::Value::Float8(num.as_f64().unwrap())),

            _ => Err(Error::TypeMismatch(value.clone(), scalar_type)),
        },

        // booleans
        serde_json::Value::Bool(b) => match scalar_type {
            database::ScalarType::Boolean => Ok(sql::ast::Value::Bool(*b)),

            _ => Err(Error::TypeMismatch(value.clone(), scalar_type)),
        },

        // strings
        serde_json::Value::String(s) => match scalar_type {
            // strings
            database::ScalarType::Character => Ok(sql::ast::Value::String(s.to_string())),
            database::ScalarType::CharacterVarying => Ok(sql::ast::Value::String(s.to_string())),
            database::ScalarType::Text => Ok(sql::ast::Value::String(s.to_string())),

            // numbers - for when the user wants to pass numbers as strings
            database::ScalarType::Smallint => Ok(sql::ast::Value::Cast {
                value: s.to_string(),
                r#type: sql::ast::ScalarType::Smallint,
            }),

            database::ScalarType::Integer => Ok(sql::ast::Value::Cast {
                value: s.to_string(),
                r#type: sql::ast::ScalarType::Integer,
            }),

            database::ScalarType::Bigint => Ok(sql::ast::Value::Cast {
                value: s.to_string(),
                r#type: sql::ast::ScalarType::Bigint,
            }),

            database::ScalarType::Real => Ok(sql::ast::Value::Cast {
                value: s.to_string(),
                r#type: sql::ast::ScalarType::Real,
            }),

            database::ScalarType::DoublePrecision => Ok(sql::ast::Value::Cast {
                value: s.to_string(),
                r#type: sql::ast::ScalarType::DoublePrecision,
            }),

            database::ScalarType::Numeric => Ok(sql::ast::Value::Cast {
                value: s.to_string(),
                r#type: sql::ast::ScalarType::Numeric,
            }),

            // date and time
            database::ScalarType::Date => Ok(sql::ast::Value::Cast {
                value: s.to_string(),
                r#type: sql::ast::ScalarType::Date,
            }),

            database::ScalarType::TimeWithTimeZone => Ok(sql::ast::Value::Cast {
                value: s.to_string(),
                r#type: sql::ast::ScalarType::TimeWithTimeZone,
            }),

            database::ScalarType::TimeWithoutTimeZone => Ok(sql::ast::Value::Cast {
                value: s.to_string(),
                r#type: sql::ast::ScalarType::TimeWithoutTimeZone,
            }),

            database::ScalarType::TimestampWithTimeZone => Ok(sql::ast::Value::Cast {
                value: s.to_string(),
                r#type: sql::ast::ScalarType::TimestampWithTimeZone,
            }),

            database::ScalarType::TimestampWithoutTimeZone => Ok(sql::ast::Value::Cast {
                value: s.to_string(),
                r#type: sql::ast::ScalarType::TimestampWithoutTimeZone,
            }),

            // uuid
            database::ScalarType::Uuid => Ok(sql::ast::Value::Cast {
                value: s.to_string(),
                r#type: sql::ast::ScalarType::Uuid,
            }),

            // Any will be passed as string for now
            database::ScalarType::Any => Ok(sql::ast::Value::String(s.to_string())),

            _ => Err(Error::TypeMismatch(value.clone(), scalar_type)),
        },
        // null
        serde_json::Value::Null => Ok(sql::ast::Value::Null),

        // not supported
        serde_json::Value::Array(_) => Err(Error::NotSupported("array values".to_string())),
        serde_json::Value::Object(_) => Err(Error::NotSupported("object values".to_string())),
    }
}
