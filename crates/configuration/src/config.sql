-- SELECT
--     TABLE_CONSTRAINTS.TABLE_NAME,
--     fc.COLUMN_NAME AS COLUMN_NAME,
--     CONSTRAINT_COLUMN_USAGE.CONSTRAINT_SCHEMA,
--     CONSTRAINT_COLUMN_USAGE.TABLE_NAME AS FOREIGN_TABLE,
--     CONSTRAINT_COLUMN_USAGE.COLUMN_NAME AS FOREIGN_COLUMN_NAME
--   FROM
--     `hasura-development.chinook_sample.INFORMATION_SCHEMA.TABLE_CONSTRAINTS` AS TABLE_CONSTRAINTS
--     INNER JOIN `hasura-development.chinook_sample.INFORMATION_SCHEMA.CONSTRAINT_COLUMN_USAGE` AS CONSTRAINT_COLUMN_USAGE ON TABLE_CONSTRAINTS.CONSTRAINT_NAME = CONSTRAINT_COLUMN_USAGE.CONSTRAINT_NAME
--     INNER JOIN `hasura-development.chinook_sample.INFORMATION_SCHEMA.KEY_COLUMN_USAGE` AS fc ON TABLE_CONSTRAINTS.CONSTRAINT_NAME = fc.CONSTRAINT_NAME
--   WHERE TABLE_CONSTRAINTS.CONSTRAINT_TYPE = 'FOREIGN KEY';


select
  coalesce(tables, to_json('{}')), -- maps to `TableInfo`
from
  (
    select
      json_object(
        -- the table alias, used for looking up the table (or view, or other relation)
        t.table_name,
        json_object(
          -- the schema name
          'schema_name',
          t.table_schema,
          -- the table name
          'table_name',
          t.table_name,
          'columns',
          -- this may be empty, in which case we coalesce with an empty object
          coalesce(
            (
              Array(select
                json_object(
                  -- the column alias, used for looking up the column
                  c.column_name,
                  json_object(
                    -- the column name
                    'name',
                    c.column_name,
                    'type',
                    -- These are the types we support, mapped to "standard" aliases.
                    -- We have a similar case expression below, the two needs to be in sync.
                    case LOWER(c.data_type)
                      when 'boolean' then 'boolean'
                      when 'int16' then 'smallint'
                      when 'smallint' then 'smallint'
                      when 'int32' then 'integer'
                      when 'integer' then 'integer'
                      when 'int64' then 'bigint'
                      when 'bigint' then 'bigint'
                      when 'numeric' then 'numeric'
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
                    end,
                    'nullable',
                    case c.is_nullable when 'YES' then 'Nullable' else 'NonNullable' end
                  )
                )
              from hasura-development.chinook_sample.INFORMATION_SCHEMA.COLUMNS as c
              where
                c.table_catalog = t.table_catalog
                and c.table_schema = t.table_schema
                and c.table_name = t.table_name
          )),
            Array ( select json_object())
          ),
          -- a mapping from the uniqueness constraint aliases to their details
          'uniqueness_constraints',
          -- this may be empty, in which case we coalesce with an empty object
          coalesce(
            (
              select
                json_object(
                  -- the name of the uniqueness constraint
                  c.constraint_name,
                  -- an array (parsed as a set) of the columns present in the constraint
                  json_array(cc.column_name) -- fixme- is json array here okay?                  -- )
                )
              from hasura-development.chinook_sample.INFORMATION_SCHEMA.TABLE_CONSTRAINTS c
              join hasura-development.chinook_sample.INFORMATION_SCHEMA.CONSTRAINT_COLUMN_USAGE as cc on cc.constraint_name = c.constraint_name
              where
                c.table_catalog = t.table_catalog
                and c.table_schema = t.table_schema
                and c.table_name = t.table_name
                and c.constraint_type in ('PRIMARY KEY', 'UNIQUE')
                and cc.constraint_catalog = c.constraint_catalog
                and cc.constraint_schema = c.constraint_schema
            ),
            json_object()
          ),
          -- a mapping from the foreign relation aliases to their details
          'foreign_relations',
          -- this may be empty, in which case we coalesce with an empty object
          coalesce(
            (
              select
                json_object(
                  -- the name of the foreign key constraint
                  c.constraint_name,
                  json_object(
                    -- the name of the foreign relation
                    'foreign_table',
                      rc.table_name,
                    -- a mapping from the local columns to the foreign columns
                    'column_mapping',
                        json_object(fc.column_name, rc.column_name)
                  )
                )
              from hasura-development.chinook_sample.INFORMATION_SCHEMA.TABLE_CONSTRAINTS as c
              join hasura-development.chinook_sample.INFORMATION_SCHEMA.CONSTRAINT_COLUMN_USAGE as rc on --
                c.constraint_catalog = rc.constraint_catalog
                and c.constraint_schema = rc.constraint_schema
                and c.constraint_name = rc.constraint_name
              join hasura-development.chinook_sample.INFORMATION_SCHEMA.KEY_COLUMN_USAGE as fc ON c.constraint_name = fc.constraint_name
              where
                c.table_catalog = t.table_catalog
                and c.table_schema = t.table_schema
                and c.table_name = t.table_name
                and c.constraint_type = 'FOREIGN KEY'
            ),
            json_object()
          )
        )
      ) as tables
    from chinook_sample.INFORMATION_SCHEMA.TABLES as t
    where t.table_schema = 'chinook_sample'
  ) as _tables