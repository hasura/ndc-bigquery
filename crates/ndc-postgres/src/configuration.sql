-- This requests the table configuration from the database.
--
-- It is very large. There are inline comments in the SQL to help understand
-- what's going on.
--
-- TODO: This uses unqualified table (and view) and constraint names.
-- We will need to qualify them at some point. This makes the aliases seem
-- redundant, but they will change in the future.

-- TODO: We derive all the information from the `information_schema`. Sadly, not
-- all information is available there, most notably:
--   * aggregate function return types
-- We therefore make a set of assumptions about these. We may wish to rewrite
-- this using the `pg_catalog` tables instead.

-- this is a temporary function for mapping SQL data types to GraphQL scalar types
create function pg_temp.data_type_to_scalar_type(information_schema.character_data)
  returns varchar
  language sql
  immutable
  return
    case $1
      when 'boolean' then 'Boolean'
      when 'smallint' then 'Int'
      when 'integer' then 'Int'
      when 'bigint' then 'Int'
      when 'smallserial' then 'Int'
      when 'serial' then 'Int'
      when 'bigserial' then 'Int'
      when 'decimal' then 'Float'
      when 'numeric' then 'Float'
      when 'real' then 'Float'
      when 'double precision' then 'Float'
      when 'text' then 'String'
      when 'character varying' then 'String'
      else 'any'
    end;

select
  tables, -- maps to `TableInfo`
  aggregate_functions -- maps to `AggregateFunctions`
from
  (
    select
      json_object_agg(
        -- the table alias, used for looking up the table (or view, or other relation)
        t.table_name,
        json_build_object(
          -- the schema name
          'schema_name',
          t.table_schema,
          -- the table name
          'table_name',
          t.table_name,
          -- a mapping from column aliases to the column information
          'columns',
          -- this may be empty, in which case we coalesce with an empty object
          coalesce(
            (
              select
                json_object_agg(
                  -- the column alias, used for looking up the column
                  c.column_name,
                  json_build_object(
                    -- the column name
                    'name',
                    c.column_name,
                    'type',
                    pg_temp.data_type_to_scalar_type(c.data_type)
                  )
                )
              from information_schema.columns c
              where
                c.table_catalog = t.table_catalog
                and c.table_schema = t.table_schema
                and c.table_name = t.table_name
            ),
            json_build_object()
          ),
          -- a mapping from the uniqueness constraint aliases to their details
          'uniqueness_constraints',
          -- this may be empty, in which case we coalesce with an empty object
          coalesce(
            (
              select
                json_object_agg(
                  -- the name of the uniqueness constraint
                  c.constraint_name,
                  -- an array (parsed as a set) of the columns present in the constraint
                  (
                    select json_agg(cc.column_name)
                    from information_schema.constraint_column_usage cc
                    where
                      cc.constraint_catalog = c.constraint_catalog
                      and cc.constraint_schema = c.constraint_schema
                      and cc.constraint_name = c.constraint_name
                  )
                )
              from information_schema.table_constraints c
              where
                c.table_catalog = t.table_catalog
                and c.table_schema = t.table_schema
                and c.table_name = t.table_name
                and c.constraint_type in ('PRIMARY KEY', 'UNIQUE')
            ),
            json_build_object()
          ),
          -- a mapping from the foreign relation aliases to their details
          'foreign_relations',
          -- this may be empty, in which case we coalesce with an empty object
          coalesce(
            (
              select
                json_object_agg(
                  -- the name of the foreign key constraint
                  c.constraint_name,
                  json_build_object(
                    -- the name of the foreign relation
                    'foreign_table',
                    (
                      select tu.table_name
                      from information_schema.constraint_table_usage tu
                      where
                        tu.constraint_catalog = c.constraint_catalog
                        and tu.constraint_schema = c.constraint_schema
                        and tu.constraint_name = c.constraint_name
                    ),
                    -- a mapping from the local columns to the foreign columns
                    'column_mapping',
                    (
                      select
                        json_object_agg(fc.column_name, uc.column_name)
                      from information_schema.key_column_usage fc
                      join information_schema.key_column_usage uc
                        on fc.position_in_unique_constraint = uc.ordinal_position
                      where
                        fc.constraint_catalog = rc.constraint_catalog
                        and fc.constraint_schema = rc.constraint_schema
                        and fc.constraint_name = rc.constraint_name
                        and uc.constraint_catalog = rc.unique_constraint_catalog
                        and uc.constraint_schema = rc.unique_constraint_schema
                        and uc.constraint_name = rc.unique_constraint_name
                    )
                  )
                )
              from information_schema.table_constraints c
              join information_schema.referential_constraints rc on
                c.constraint_catalog = rc.constraint_catalog
                and c.constraint_schema = rc.constraint_schema
                and c.constraint_name = rc.constraint_name
              where
                c.table_catalog = t.table_catalog
                and c.table_schema = t.table_schema
                and c.table_name = t.table_name
                and c.constraint_type = 'FOREIGN KEY'
            ),
            json_build_object()
          )
        )
      ) tables
    from information_schema.tables t
    where t.table_schema = 'public'
  ) _tables,
  (
    select
      json_object_agg(
        -- the name of the GraphQL scalar type
        scalar_type,
        -- the set of functions
        routines
      ) aggregate_functions
    from
      (
        select
          pg_temp.data_type_to_scalar_type(data_type) scalar_type,
          json_object_agg(
            -- the function name
            routine_name,
            json_build_object(
              -- the return type of the aggregate function
              -- NOTE: this information is not actually available, so we assume
              -- the return type is always the same as the input type
              'return_type',
              pg_temp.data_type_to_scalar_type(data_type)
            )
          ) routines
        from information_schema.routines r
        where routine_schema in ('pg_catalog', 'public')
        and routine_type is null -- aggregate functions don't have a routine type
        and pg_temp.data_type_to_scalar_type(data_type) <> 'any'
        group by pg_temp.data_type_to_scalar_type(data_type)
      ) routines_by_type
  ) _aggregate_functions
