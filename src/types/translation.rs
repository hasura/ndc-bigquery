pub struct ExecutionPlan {
    pub root_field: String,
    pub pre: Vec<DDL>,
    pub query: Query,
    pub post: Vec<DDL>,
}
pub struct SQL {
    pub sql: String,
    pub params: Vec<SQLValue>,
}
pub struct DDL(pub SQL);
pub struct Query(pub SQL);
pub enum SQLValue {
    Int4(i32),
}
