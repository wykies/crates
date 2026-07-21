use anyhow::Context as _;
use anyhow::bail;
use jiff::civil;
use umya_spreadsheet::helper::date::excel_to_date_time_jiff;

#[inline]
pub fn str_to_u8(cell_value: &str, value_name: &str) -> anyhow::Result<u8> {
    cell_value
        .parse()
        .with_context(|| format!("failed to convert {value_name:?} to u8. Value: '{cell_value}'"))
}

#[inline]
pub fn str_to_bool(cell_value: &str) -> anyhow::Result<bool> {
    match cell_value {
        "TRUE" => Ok(true),
        "FALSE" => Ok(false),
        other => bail!("failed to convert value {other:?} into bool"),
    }
}

#[inline]
pub fn f64_str_to_date_time(value: &str, value_name: &str) -> anyhow::Result<civil::DateTime> {
    let value: f64 = value.parse().with_context(|| {
        format!("failed to convert to numeric value for date of {value_name:?} on value: {value:?}")
    })?;
    Ok(excel_to_date_time_jiff(value))
}
