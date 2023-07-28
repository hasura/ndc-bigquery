# General architecture

## Query Engine

The query engine's job is to take a `QueryRequest`, which contains information about the query a user would like to run,
translate it to PostgreSQL SQL, execute it against the database, and return the results as a `QueryReponse`.

One place in particular that uses the Query Engine is the `/query` endpoint (defined in the `ndc-hub` repository).

`/query` endpoints receives a `QueryRequest`, and calls the `translate` function from the Query Engine
with it and with the information about the tables tracked in the metadata to receive and `ExecutionPlan`.
It then calls the `execute` function from the Query Engine with the same `ExecutionPlan`
(which then runs it against postgres) and gets back a `QueryResponse` which it can then return to the caller.

API:

```rs
pub fn translate(
    tables_info: &metadata::TablesInfo,
    query_request: models::QueryRequest,
) -> Result<ExecutionPlan, Error>
```

```rs
pub async fn execute(
    pool: sqlx::PgPool,
    plan: translation::ExecutionPlan,
) -> Result<models::QueryResponse, sqlx::Error>
```


### Translation

The translation step is essentially side-effect free - we use information from the request, as well as the information
about the metadata to translate the query request into steps to run against the database.

This process is currently found in the [phases/translation.rs](/crates/query-engine/src/phases/translation.rs) file, and the API
is the following function:

```rs
pub fn translate(
    tables_info: &metadata::TablesInfo,
    query_request: models::QueryRequest,
) -> Result<ExecutionPlan, Error>
```

The `translate` function returns a `ExecutionPlan`.

```rs
pub struct ExecutionPlan {
    pub root_field: String,
    /// Run before the query. Should be a sql_ast in the future.
    pub pre: Vec<sql_string::DDL>,
    /// The query.
    pub query: sql_ast::Select,
    /// Run after the query. Should be a sql_ast in the future.
    pub post: Vec<sql_string::DDL>,
}
```

Right now we don't expect `pre` and `post` to be populated, but it could be used for things like Stored Procedures.

### SQL AST

We maintain a SQL AST represented as Rust data types in [phases/translation/sql_ast.rs](/crates/query-engine/src/phases/translation/sql_ast.rs).
We implement our own representation because we want more control over this core component of our application,
and we want to implement exactly what we want and not more or less. Other external libraries such as `sqlparser`
do not have the same goals as us, and we'll have to make compromises that will affect our codebase's complexity
and development velocity.

We have a few guidelines for the SQL AST:

#### The SQL AST should mimic PostgreSQL directly

The SQL AST should look like a subset of PostgreSQL SQL, and not contain any high-level abstractions, or try to abstract
multiple SQL ASTs. We should implement exactly what we need, and be precise about it.

Should we need a higher-level of abstraction, and additional IR should be constructed that will sit before the SQL AST.

#### Implement what you need and not more

The SQL AST should contain structures we actively use, and not contain structures we don't use.

One such example is window functions - we don't need to include them in the AST currently because we don't have features
that use them from GraphQL.

### SQL string

The SQL string is a stringify representation of the SQL AST. It can be found in [phases/translation/sql_string.rs](/crates/query-engine/src/phases/translation/sql_string.rs).

We separate the SQL to AST and string representation so we can write transformations and optimizations on the SQL AST.

The SQL string representation should be generated from the SQL AST by pretty printing the result.
The result of converting ([phases/translation/convert.rs](/crates/query-engine/src/phases/translation/convert.rs)) a sql ast to string should produce
a query string that can be run against postgres as a parameterized query, as well as the parameters that are supplied
by the user.

### Query Execution

The query execution receives a pool and a plan and executes it against postgres. It then returns the results from the query part
back to the caller of the function.
The code can be found in [phases/execution.rs](/crates/query-engine/src/phases/execution.rs)


```rs
pub async fn execute(
    pool: sqlx::PgPool,
    plan: translation::ExecutionPlan,
) -> Result<models::QueryResponse, sqlx::Error>
```
