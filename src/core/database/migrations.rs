use sqlx::Acquire;
use crate::error::ZhangResult;
use sqlx::sqlite::SqliteConnection;

pub struct Migration;


static TABLES: [&'static str; 11] = ["options", "accounts", "metas", "commodities", "documents",
    "transactions", "transaction_links", "transaction_tags", "transaction_postings",
    "prices", "commodity_lots"
    ];

static TABLES_SQL: [&'static str; 12] = [
    r#"
    create table if not exists options
    (
        key   varchar not null
            primary key,
        value varchar
    );
    "#,

    r#"
    create table if not exists prices
    (
        datetime         datetime not null,
        commodity        varchar  not null,
        amount           numeric  not null,
        target_commodity varchar  not null
    );
    "#,

    r#"
    create table if not exists accounts
    (
        date   datetime not null,
        name   varchar  not null
            primary key,
        status varchar  not null,
        alias  varchar
    );
    "#,

    r#"
    create table if not exists metas
    (
        type            varchar not null,
        type_identifier varchar not null,
        key             varchar not null,
        value           varchar
    );
    "#,

    r#"
    create table if not exists commodities
    (
        name      varchar not null
            constraint commodities_pk
                primary key,
        precision INTEGER,
        prefix    varchar,
        suffix    varchar,
        rounding  varchar
    );
    "#,

    r#"
    create table if not exists commodity_lots
    (
        commodity       varchar not null,
        datetime        datetime,
        amount          REAL,
        price_amount    REAL,
        price_commodity varchar,
        account         varchar
    );
    "#,

    r#"
    create table if not exists documents
    (
        datetime  datetime not null,
        filename  varchar  not null,
        path      varchar  not null,
        extension varchar,
        account   varchar,
        trx_id    varchar
    );
    "#,

    r#"
    create table if not exists transactions
    (
        id        varchar  not null
            primary key
            unique,
        datetime  datetime not null,
        type      varchar,
        payee     varchar,
        narration varchar
    );
    "#,

    r#"
    create table if not exists transaction_links
    (
        trx_id varchar not null,
        link   varchar not null
    );
    "#,

    r#"
    create table if not exists transaction_tags
    (
        trx_id varchar not null,
        tag    varchar not null
    );
    "#,

    r#"
    create table if not exists transaction_postings
    (
        trx_id                   varchar not null,
        account                  varchar not null,
        unit_number              REAL,
        unit_commodity           varchar,
        cost_number              REAL,
        cost_commodity           varchar,
        price_number             REAL,
        price_commodity          varchar,
        inferred_unit_number     REAL,
        inferred_unit_commodity  varchar,
        account_before_number    REAL,
        account_before_commodity varchar,
        account_after_number     REAL,
        account_after_commodity  varchar
    );
    "#,

    r#"
    CREATE VIEW if not exists account_balance as
    select transactions.datetime,
           account_max_datetime.account,
           account_after_number,
           transaction_postings.account_after_commodity
    from transactions
             join transaction_postings on transactions.id = transaction_postings.trx_id

             join (select max(datetime) as max_datetime, account, account_after_commodity
                   from transaction_postings
                            join transactions on transactions.id = transaction_postings.trx_id
                   group by account, account_after_commodity) account_max_datetime
                  on transactions.datetime = account_max_datetime.max_datetime and
                     transaction_postings.account = account_max_datetime.account
                      and transaction_postings.account_after_commodity = account_max_datetime.account_after_commodity;
    "#,
];



impl Migration {
    pub async fn init_database_if_missing(conn: &mut SqliteConnection) -> ZhangResult<()> {
        Migration::clear_tables(conn).await?;

        let mut trx = conn.begin().await?;
        for sql in TABLES_SQL {
            sqlx::query(sql)
                .execute(&mut trx)
                .await?;
        }
        trx.commit().await?;

        Ok(())
    }
    pub async fn clear_tables(conn: &mut SqliteConnection) -> ZhangResult<()> {
        let mut trx = conn.begin().await?;

        for table_name in TABLES {
            sqlx::query(&format!("DROP TABLE IF EXISTS {table_name}"))
            // sqlx::query(&format!("delete from {table_name}"))
                .execute(&mut trx)
                .await?;
        }
        trx.commit().await?;
        Ok(())
    }
}