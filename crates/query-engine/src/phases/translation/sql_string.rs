/// Type definitions of a low-level SQL string representation.
#[derive(Debug, PartialEq, Eq)]
pub struct SQL {
    pub sql: String,
    pub params: Vec<Param>,
    /// for internal use and tests only
    pub param_index: u64,
}

impl Default for SQL {
    fn default() -> Self {
        Self::new()
    }
}

/// A parameter for a parameterized query.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Param {
    /// A literal string
    String(String),
    /// A variable name to look up in the `variables` field in a `QueryRequest`.
    Variable(String),
}

/// A DDL statement.
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
    pub fn append_param(&mut self, param: Param) {
        self.param_index += 1;
        self.sql.push_str(format!("${}", self.param_index).as_str());
        self.params.push(param);
    }
}
