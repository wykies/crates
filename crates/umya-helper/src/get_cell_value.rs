use anyhow::Context as _;
use jiff::civil;
use std::borrow::Cow;
use std::fmt::Debug;
use tracing::instrument;
use umya_spreadsheet::Worksheet;
use umya_spreadsheet::helper::coordinate::CellCoordinates;

use crate::{f64_str_to_date_time, str_to_bool, str_to_u8};

/// Only returns the cell value if it exists and it's not empty
pub fn get_cell_value<T>(sheet: &Worksheet, coordinate: T) -> Option<Cow<'static, str>>
where
    T: Into<CellCoordinates> + Debug,
{
    let x = sheet.cell(coordinate)?;
    let value = x.value();
    if value.trim().is_empty() {
        None
    } else {
        Some(value)
    }
}

#[inline]
#[instrument(ret, err(Debug), level = "debug", fields(sheet_name = sheet.name()), skip(sheet))]
pub fn get_expected_cell_value<T>(
    sheet: &Worksheet,
    coordinate: T,
    value_name: &str,
) -> anyhow::Result<Cow<'static, str>>
where
    T: Into<CellCoordinates> + Debug,
{
    Ok(sheet
        .cell(coordinate)
        .with_context(|| format!("failed to get {value_name:?}"))?
        .value())
}

#[inline]
pub fn get_expected_cell_value_as_date_time<T>(
    sheet: &Worksheet,
    coordinate: T,
    value_name: &str,
) -> anyhow::Result<civil::DateTime>
where
    T: Into<CellCoordinates> + Debug,
{
    let value = get_expected_cell_value(sheet, coordinate, value_name)?;
    f64_str_to_date_time(&value, value_name)
}

#[inline]
pub fn get_expected_cell_value_as_time<T>(
    sheet: &Worksheet,
    coordinate: T,
    value_name: &str,
) -> anyhow::Result<civil::Time>
where
    T: Into<CellCoordinates> + Debug,
{
    Ok(get_expected_cell_value_as_date_time(sheet, coordinate, value_name)?.time())
}

#[inline]
pub fn get_expected_cell_value_as_date<T>(
    sheet: &Worksheet,
    coordinate: T,
    value_name: &str,
) -> anyhow::Result<civil::Date>
where
    T: Into<CellCoordinates> + Debug,
{
    Ok(get_expected_cell_value_as_date_time(sheet, coordinate, value_name)?.date())
}

#[inline]
pub fn get_expected_cell_value_as_bool<T>(
    sheet: &Worksheet,
    coordinate: T,
    value_name: &str,
) -> anyhow::Result<bool>
where
    T: Into<CellCoordinates> + Debug,
{
    let cell_value = get_expected_cell_value(sheet, coordinate, value_name)?;
    str_to_bool(&cell_value)
}

#[inline]
pub fn get_cell_value_as_f64<T>(sheet: &Worksheet, coordinate: T) -> Option<f64>
where
    T: Into<CellCoordinates> + Debug,
{
    sheet.cell(coordinate)?.value_number()
}

#[inline]
pub fn get_expected_cell_value_as_f64<T>(
    sheet: &Worksheet,
    coordinate: T,
    value_name: &str,
) -> anyhow::Result<f64>
where
    T: Into<CellCoordinates> + Debug,
{
    sheet
        .cell(coordinate)
        .with_context(|| format!("failed to get cell for {value_name:?}"))?
        .value_number()
        .with_context(|| format!("failed to get f64 for {value_name:?}"))
}

#[inline]
pub fn get_cell_value_as_u8<T>(
    sheet: &Worksheet,
    coordinate: T,
    value_name: &str,
) -> Option<anyhow::Result<u8>>
where
    T: Into<CellCoordinates> + Debug,
{
    let cell_value = get_cell_value(sheet, coordinate)?;
    Some(str_to_u8(&cell_value, value_name))
}
