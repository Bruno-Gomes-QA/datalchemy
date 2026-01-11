use sqlx::PgPool;

use datalchemy_core::Result;

pub async fn fetch_database_name(pool: &PgPool) -> Result<String> {
    let name = sqlx::query_scalar::<_, String>("select current_database()")
        .fetch_one(pool)
        .await
        .map_err(|err| datalchemy_core::Error::Db(err.to_string()))?;
    Ok(name)
}

pub async fn list_schemas(pool: &PgPool) -> Result<Vec<String>> {
    let rows = sqlx::query!(
        r#"
        select nspname as "name!"
        from pg_namespace
        order by nspname
        "#
    )
    .fetch_all(pool)
    .await
    .map_err(|err| datalchemy_core::Error::Db(err.to_string()))?;

    Ok(rows.into_iter().map(|row| row.name).collect())
}

pub struct RawTable {
    pub name: String,
    pub relkind: i8,
    pub comment: Option<String>,
}

pub async fn list_tables_in_schema(pool: &PgPool, schema: &str) -> Result<Vec<RawTable>> {
    let rows = sqlx::query!(
        r#"
        select
          c.relname as "name!",
          c.relkind as "relkind!",
          pg_catalog.obj_description(c.oid, 'pg_class') as "comment"
        from pg_class c
        join pg_namespace n on n.oid = c.relnamespace
        where n.nspname = $1
          and c.relkind in ('r','p','v','m','f')
        order by c.relname
        "#,
        schema
    )
    .fetch_all(pool)
    .await
    .map_err(|err| datalchemy_core::Error::Db(err.to_string()))?;

    Ok(rows
        .into_iter()
        .map(|row| RawTable {
            name: row.name,
            relkind: row.relkind,
            comment: row.comment,
        })
        .collect())
}

pub struct RawColumn {
    pub ordinal_position: i16,
    pub name: String,
    pub data_type: String,
    pub udt_schema: String,
    pub udt_name: String,
    pub is_nullable: bool,
    pub default: Option<String>,
    pub identity_generation: Option<String>,
    pub is_generated: bool,
    pub generation_expression: Option<String>,
    pub character_max_length: Option<i32>,
    pub numeric_precision: Option<i32>,
    pub numeric_scale: Option<i32>,
    pub collation: Option<String>,
    pub comment: Option<String>,
}

pub async fn list_columns(pool: &PgPool, schema: &str, table: &str) -> Result<Vec<RawColumn>> {
    let rows = sqlx::query!(
        r#"
        select
          a.attnum as "ordinal_position!",
          a.attname as "name!",
          pg_catalog.format_type(a.atttypid, a.atttypmod) as "data_type!",
          tn.nspname as "udt_schema!",
          t.typname as "udt_name!",
          (not a.attnotnull) as "is_nullable!",
          pg_get_expr(ad.adbin, ad.adrelid) as "default",
          case
            when a.attidentity = '' then null
            when a.attidentity = 'a' then 'ALWAYS'
            when a.attidentity = 'd' then 'BY DEFAULT'
            else null
          end as "identity_generation?",
          (a.attgenerated <> '') as "is_generated!",
          case
            when a.attgenerated <> '' then pg_get_expr(ad.adbin, ad.adrelid)
            else null
          end as "generation_expression?",
          ic.character_maximum_length as "character_max_length",
          ic.numeric_precision as "numeric_precision",
          ic.numeric_scale as "numeric_scale",
          ic.collation_name as "collation",
          pg_catalog.col_description(a.attrelid, a.attnum) as "comment"
        from pg_attribute a
        join pg_class c on c.oid = a.attrelid
        join pg_namespace n on n.oid = c.relnamespace
        join pg_type t on t.oid = a.atttypid
        join pg_namespace tn on tn.oid = t.typnamespace
        left join pg_attrdef ad on ad.adrelid = a.attrelid and ad.adnum = a.attnum
        left join information_schema.columns ic
          on ic.table_schema = n.nspname and ic.table_name = c.relname and ic.column_name = a.attname
        where n.nspname = $1
          and c.relname = $2
          and a.attnum > 0
          and not a.attisdropped
        order by a.attnum
        "#,
        schema,
        table
    )
    .fetch_all(pool)
    .await
    .map_err(|err| datalchemy_core::Error::Db(err.to_string()))?;

    Ok(rows
        .into_iter()
        .map(|row| RawColumn {
            ordinal_position: row.ordinal_position,
            name: row.name,
            data_type: row.data_type,
            udt_schema: row.udt_schema,
            udt_name: row.udt_name,
            is_nullable: row.is_nullable,
            default: row.default,
            identity_generation: row.identity_generation,
            is_generated: row.is_generated,
            generation_expression: row.generation_expression,
            character_max_length: row.character_max_length,
            numeric_precision: row.numeric_precision,
            numeric_scale: row.numeric_scale,
            collation: row.collation,
            comment: row.comment,
        })
        .collect())
}

pub struct RawPrimaryKey {
    pub name: String,
    pub columns: Vec<String>,
}

pub async fn get_primary_key(
    pool: &PgPool,
    schema: &str,
    table: &str,
) -> Result<Option<RawPrimaryKey>> {
    let row = sqlx::query!(
        r#"
        select
          con.conname as "name!",
          array_agg(att.attname order by ord.ordinality) as "columns!"
        from pg_constraint con
        join pg_class rel on rel.oid = con.conrelid
        join pg_namespace nsp on nsp.oid = rel.relnamespace
        join unnest(con.conkey) with ordinality as ord(attnum, ordinality) on true
        join pg_attribute att on att.attrelid = rel.oid and att.attnum = ord.attnum
        where nsp.nspname = $1
          and rel.relname = $2
          and con.contype = 'p'
        group by con.conname
        "#,
        schema,
        table
    )
    .fetch_optional(pool)
    .await
    .map_err(|err| datalchemy_core::Error::Db(err.to_string()))?;

    Ok(row.map(|row| RawPrimaryKey {
        name: row.name,
        columns: row.columns,
    }))
}

pub struct RawUniqueConstraint {
    pub name: String,
    pub columns: Vec<String>,
    pub is_deferrable: bool,
    pub initially_deferred: bool,
}

pub async fn list_unique_constraints(
    pool: &PgPool,
    schema: &str,
    table: &str,
) -> Result<Vec<RawUniqueConstraint>> {
    let rows = sqlx::query!(
        r#"
        select
          con.conname as "name!",
          array_agg(att.attname order by ord.ordinality) as "columns!",
          con.condeferrable as "is_deferrable!",
          con.condeferred as "initially_deferred!"
        from pg_constraint con
        join pg_class rel on rel.oid = con.conrelid
        join pg_namespace nsp on nsp.oid = rel.relnamespace
        join unnest(con.conkey) with ordinality as ord(attnum, ordinality) on true
        join pg_attribute att on att.attrelid = rel.oid and att.attnum = ord.attnum
        where nsp.nspname = $1
          and rel.relname = $2
          and con.contype = 'u'
        group by con.conname, con.condeferrable, con.condeferred
        order by con.conname
        "#,
        schema,
        table
    )
    .fetch_all(pool)
    .await
    .map_err(|err| datalchemy_core::Error::Db(err.to_string()))?;

    Ok(rows
        .into_iter()
        .map(|row| RawUniqueConstraint {
            name: row.name,
            columns: row.columns,
            is_deferrable: row.is_deferrable,
            initially_deferred: row.initially_deferred,
        })
        .collect())
}

pub struct RawCheckConstraint {
    pub name: String,
    pub expression: String,
}

pub async fn list_check_constraints(
    pool: &PgPool,
    schema: &str,
    table: &str,
) -> Result<Vec<RawCheckConstraint>> {
    let rows = sqlx::query!(
        r#"
        select
          con.conname as "name!",
          pg_get_constraintdef(con.oid, true) as "expression!"
        from pg_constraint con
        join pg_class rel on rel.oid = con.conrelid
        join pg_namespace nsp on nsp.oid = rel.relnamespace
        where nsp.nspname = $1
          and rel.relname = $2
          and con.contype = 'c'
        order by con.conname
        "#,
        schema,
        table
    )
    .fetch_all(pool)
    .await
    .map_err(|err| datalchemy_core::Error::Db(err.to_string()))?;

    Ok(rows
        .into_iter()
        .map(|row| RawCheckConstraint {
            name: row.name,
            expression: row.expression,
        })
        .collect())
}

pub struct RawForeignKey {
    pub name: String,
    pub columns: Vec<String>,
    pub referenced_schema: String,
    pub referenced_table: String,
    pub referenced_columns: Vec<String>,
    pub on_update_code: i8,
    pub on_delete_code: i8,
    pub match_type_code: i8,
    pub is_deferrable: bool,
    pub initially_deferred: bool,
}

pub async fn list_foreign_keys(
    pool: &PgPool,
    schema: &str,
    table: &str,
) -> Result<Vec<RawForeignKey>> {
    let rows = sqlx::query!(
        r#"
        select
          con.conname as "name!",
          array_agg(src_att.attname order by s_ord.ordinality) as "columns!",
          ref_nsp.nspname as "referenced_schema!",
          ref_rel.relname as "referenced_table!",
          array_agg(ref_att.attname order by t_ord.ordinality) as "referenced_columns!",
          con.confupdtype as "on_update_code!",
          con.confdeltype as "on_delete_code!",
          con.confmatchtype as "match_type_code!",
          con.condeferrable as "is_deferrable!",
          con.condeferred as "initially_deferred!"
        from pg_constraint con
        join pg_class src_rel on src_rel.oid = con.conrelid
        join pg_namespace src_nsp on src_nsp.oid = src_rel.relnamespace
        join pg_class ref_rel on ref_rel.oid = con.confrelid
        join pg_namespace ref_nsp on ref_nsp.oid = ref_rel.relnamespace
        join unnest(con.conkey) with ordinality as s_ord(attnum, ordinality) on true
        join pg_attribute src_att on src_att.attrelid = src_rel.oid and src_att.attnum = s_ord.attnum
        join unnest(con.confkey) with ordinality as t_ord(attnum, ordinality) on true
        join pg_attribute ref_att on ref_att.attrelid = ref_rel.oid and ref_att.attnum = t_ord.attnum
        where src_nsp.nspname = $1
          and src_rel.relname = $2
          and con.contype = 'f'
        group by
          con.conname, ref_nsp.nspname, ref_rel.relname,
          con.confupdtype, con.confdeltype, con.confmatchtype,
          con.condeferrable, con.condeferred
        order by con.conname
        "#,
        schema,
        table
    )
    .fetch_all(pool)
    .await
    .map_err(|err| datalchemy_core::Error::Db(err.to_string()))?;

    Ok(rows
        .into_iter()
        .map(|row| RawForeignKey {
            name: row.name,
            columns: row.columns,
            referenced_schema: row.referenced_schema,
            referenced_table: row.referenced_table,
            referenced_columns: row.referenced_columns,
            on_update_code: row.on_update_code,
            on_delete_code: row.on_delete_code,
            match_type_code: row.match_type_code,
            is_deferrable: row.is_deferrable,
            initially_deferred: row.initially_deferred,
        })
        .collect())
}

pub struct RawIndex {
    pub name: String,
    pub is_unique: bool,
    pub is_primary: bool,
    pub is_valid: bool,
    pub method: String,
    pub definition: String,
}

pub async fn list_indexes(pool: &PgPool, schema: &str, table: &str) -> Result<Vec<RawIndex>> {
    let rows = sqlx::query!(
        r#"
        select
          idx.relname as "name!",
          i.indisunique as "is_unique!",
          i.indisprimary as "is_primary!",
          i.indisvalid as "is_valid!",
          am.amname as "method!",
          pg_get_indexdef(i.indexrelid) as "definition!"
        from pg_index i
        join pg_class tbl on tbl.oid = i.indrelid
        join pg_namespace nsp on nsp.oid = tbl.relnamespace
        join pg_class idx on idx.oid = i.indexrelid
        join pg_am am on am.oid = idx.relam
        where nsp.nspname = $1
          and tbl.relname = $2
        order by idx.relname
        "#,
        schema,
        table
    )
    .fetch_all(pool)
    .await
    .map_err(|err| datalchemy_core::Error::Db(err.to_string()))?;

    Ok(rows
        .into_iter()
        .map(|row| RawIndex {
            name: row.name,
            is_unique: row.is_unique,
            is_primary: row.is_primary,
            is_valid: row.is_valid,
            method: row.method,
            definition: row.definition,
        })
        .collect())
}

pub struct RawEnumType {
    pub schema: String,
    pub name: String,
    pub labels: Vec<String>,
}

pub async fn list_enums(pool: &PgPool) -> Result<Vec<RawEnumType>> {
    let rows = sqlx::query!(
        r#"
        select
          n.nspname as "schema!",
          t.typname as "name!",
          array_agg(e.enumlabel order by e.enumsortorder) as "labels!"
        from pg_type t
        join pg_namespace n on n.oid = t.typnamespace
        join pg_enum e on e.enumtypid = t.oid
        group by n.nspname, t.typname
        order by n.nspname, t.typname
        "#
    )
    .fetch_all(pool)
    .await
    .map_err(|err| datalchemy_core::Error::Db(err.to_string()))?;

    Ok(rows
        .into_iter()
        .map(|row| RawEnumType {
            schema: row.schema,
            name: row.name,
            labels: row.labels,
        })
        .collect())
}
