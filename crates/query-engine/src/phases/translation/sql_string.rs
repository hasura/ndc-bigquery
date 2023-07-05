/// Type definitions of a low-level SQL string representation.
use super::sql_ast::Value;

#[derive(Debug, PartialEq, Eq)]
pub struct SQL {
    pub sql: String,
    pub params: Vec<Value>,
    /// for internal use and tests only
    pub param_index: u64,
}

impl Default for SQL {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug)]
pub struct DDL(pub SQL);

impl SQL {
    pub fn new() -> SQL {
        SQL {
            sql: "".to_string(),
            params: vec![],
            param_index: 0,
        }
    }
    pub fn append_syntax(&mut self, sql: &str) {
        self.sql.push_str(sql);
    }
    pub fn append_identifier(&mut self, sql: &String) {
        // todo: sanitize
        self.sql.push_str(format!("\"{}\"", sql).as_str());
    }
    pub fn append_param(&mut self, value: Value) {
        self.param_index += 1;
        self.sql.push_str(format!("${}", self.param_index).as_str());
        self.params.push(value);
    }
}
