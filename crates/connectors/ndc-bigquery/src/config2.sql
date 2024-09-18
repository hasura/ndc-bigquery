WITH column_data AS (
  SELECT
    t.table_name,
    t.table_catalog,
    t.table_schema,
    c.column_name,
    TO_JSON_STRING(STRUCT(
      c.column_name AS name,
      JSON_OBJECT('ScalarType', 
        case LOWER(c.data_type)
                  when 'boolean' then 'boolean'
                  when 'int16' then 'smallint'
                  when 'smallint' then 'smallint'
                  when 'int32' then 'integer'
                  when 'integer' then 'integer'
                  when 'int64' then 'bigint'
                  when 'bigint' then 'bigint'
                  when 'numeric' then 'numeric'
                  when 'float64' then 'float'
                  when 'float' then 'float'
                  when 'real' then 'real'
                  when 'double precision' then 'double precision'
                  when 'text' then 'text'
                  when 'string' then 'string'
                  when 'character' then 'character'
                  when 'json' then 'json'
                  when 'jsonb' then 'jsonb'
                  when 'date' then 'date'
                  when 'time with time zone' then 'time with time zone'
                  when 'time without time zone' then 'time without time zone'
                  when 'timestamp with time zone' then 'timestamp with time zone'
                  when 'timestamp without time zone' then 'timestamp without time zone'
                  when 'uuid' then 'uuid'
                  else 'any'
                end
        ) AS type,
      CASE WHEN c.is_nullable = 'YES' THEN 'nullable' ELSE 'nonNullable' END AS nullable
    )) AS column_info
  FROM HASURA_DATABASE_NAME_PLACEHOLDER.INFORMATION_SCHEMA.TABLES AS t
  JOIN HASURA_DATABASE_NAME_PLACEHOLDER.INFORMATION_SCHEMA.COLUMNS AS c
    ON c.table_catalog = t.table_catalog
    AND c.table_schema = t.table_schema
    AND c.table_name = t.table_name
  WHERE t.table_schema = 'chinook_sample'
),
columns_struct AS (
  SELECT
    table_name,
    table_catalog,
    table_schema,
    STRUCT(
      STRING_AGG(
        CONCAT('"', column_name, '":', column_info),
        ','
      ) AS columns_json
    ) AS columns
  FROM column_data
  GROUP BY table_name, table_catalog, table_schema
),
relationship_data AS (
  SELECT
    t.table_name,
    t.table_catalog,
    t.table_schema,
    c.constraint_name,
    TO_JSON_STRING(STRUCT(
        rc.table_name AS foreign_table,
        json_object(fc.column_name, rc.column_name) as column_mapping
    )) AS relationship_info
  FROM HASURA_DATABASE_NAME_PLACEHOLDER.INFORMATION_SCHEMA.TABLES AS t
  JOIN HASURA_DATABASE_NAME_PLACEHOLDER.INFORMATION_SCHEMA.TABLE_CONSTRAINTS as c
    ON c.table_catalog = t.table_catalog
    AND c.table_schema = t.table_schema
    AND c.table_name = t.table_name
  JOIN HASURA_DATABASE_NAME_PLACEHOLDER.INFORMATION_SCHEMA.CONSTRAINT_COLUMN_USAGE as rc
    ON c.constraint_catalog = rc.constraint_catalog
    AND c.constraint_schema = rc.constraint_schema
    AND c.constraint_name = rc.constraint_name
  JOIN HASURA_DATABASE_NAME_PLACEHOLDER.INFORMATION_SCHEMA.KEY_COLUMN_USAGE as fc ON c.constraint_name = fc.constraint_name
  WHERE t.table_schema = 'chinook_sample' AND c.constraint_type = 'FOREIGN KEY'
  GROUP BY t.table_name, table_catalog, table_schema, constraint_name, rc.table_name, fc.column_name, rc.column_name
),
relationship_struct AS (
  SELECT
    table_name,
    table_catalog,
    table_schema,
    STRUCT(
      STRING_AGG(
        CONCAT('"', constraint_name, '":', relationship_info),
        ','
      ) AS relationships_json
    ) AS relationships
  FROM relationship_data
  GROUP BY table_name, table_catalog, table_schema
),
unique_constraint_data AS (
  SELECT
    t.table_name,
    t.table_catalog,
    t.table_schema,
    c.constraint_name,
    TO_JSON_STRING(JSON_ARRAY(cc.column_name)) AS unique_constraint_info
  FROM HASURA_DATABASE_NAME_PLACEHOLDER.INFORMATION_SCHEMA.TABLES AS t
  JOIN HASURA_DATABASE_NAME_PLACEHOLDER.INFORMATION_SCHEMA.TABLE_CONSTRAINTS as c
    ON c.table_catalog = t.table_catalog
    AND c.table_schema = t.table_schema
    AND c.table_name = t.table_name
  JOIN HASURA_DATABASE_NAME_PLACEHOLDER.INFORMATION_SCHEMA.CONSTRAINT_COLUMN_USAGE as cc
    ON c.constraint_name = cc.constraint_name
  WHERE t.table_schema = 'chinook_sample' 
    AND c.constraint_type in ('PRIMARY KEY', 'UNIQUE')
    AND cc.constraint_catalog = c.constraint_catalog
    AND cc.constraint_schema = c.constraint_schema
),
unique_constraint_struct AS (
  SELECT
    table_name,
    table_catalog,
    table_schema,
    STRUCT(
      STRING_AGG(
        CONCAT('"', constraint_name, '":', unique_constraint_info),
        ','
      ) AS unique_constraint_json
    ) AS unique_constraint
  FROM unique_constraint_data
  GROUP BY table_name, table_catalog, table_schema
)
SELECT
  CONCAT('{', STRING_AGG(CONCAT(
    '"', columns_struct.table_name, '": {',
      '"schemaName": ',
      '"', CONCAT(columns_struct.table_catalog , '.', columns_struct.table_schema), '", ',
      '"tableName": ' , '"', columns_struct.table_name, '", '
      '"columns": {', 
        columns_struct.columns.columns_json,
      '},',
      '"uniquenessConstraints": {',
        coalesce(unique_constraint_struct.unique_constraint.unique_constraint_json, ""),
      '},',
      '"foreignRelations": {',
        coalesce(relationship_struct.relationships.relationships_json, ""),
      '}'
    '}'
  )), '}') AS result
FROM columns_struct 
LEFT JOIN relationship_struct ON columns_struct.table_name = relationship_struct.table_name 
LEFT JOIN unique_constraint_struct ON columns_struct.table_name = unique_constraint_struct.table_name
