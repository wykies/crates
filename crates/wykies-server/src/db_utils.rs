use anyhow::bail;

use crate::db_types::DbSqlResult;

pub fn validate_one_row_affected(sql_result: &DbSqlResult) -> anyhow::Result<()> {
    validate_rows_affected(sql_result, 1)
}

pub fn validate_rows_affected(
    sql_result: &DbSqlResult,
    expected_rows_affected: u64,
) -> anyhow::Result<()> {
    match (sql_result.rows_affected(), expected_rows_affected) {
        (actual, expected) if actual == expected => Ok(()),
        (actual, expected) => bail!("actual rows affected is {actual} but expected {expected}"),
    }
}

pub fn db_int_to_bool(value: i8) -> bool {
    value != 0
}
