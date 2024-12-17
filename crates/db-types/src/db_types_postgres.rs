pub type DbPool = sqlx::PgPool;
pub type Db = sqlx::Postgres;
pub type DbConnection = sqlx::PgConnection;
pub type DbSslMode = sqlx::postgres::PgSslMode;
pub type DbConnectOptions = sqlx::postgres::PgConnectOptions;
pub type DbPoolOptions = sqlx::postgres::PgPoolOptions;
pub type DbSqlResult = sqlx::postgres::PgQueryResult;
