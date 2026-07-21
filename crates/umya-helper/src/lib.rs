use anyhow::Context as _;
use anyhow::bail;
use jiff::civil;
use std::borrow::Cow;
use std::fmt::Debug;
use tracing::instrument;
use umya_spreadsheet::{
    Comment, RichText, TextElement, Worksheet,
    helper::{coordinate::CellCoordinates, date::excel_to_date_time_jiff},
};

pub fn get_next_empty_row(sheet: &Worksheet, start_row: u32, column_to_check: u32) -> u32 {
    (start_row..)
        .find(|&row| {
            sheet
                .cell((column_to_check, row))
                .is_none_or(|x| x.value().trim().is_empty())
        })
        .expect("runs on an infinite iterator and can only exit with a value")
}

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

#[inline]
pub fn set_cell_value<S: Into<String>>(sheet: &mut Worksheet, row: u32, col: u32, value: S) {
    sheet.cell_mut((col, row)).set_value(value);
}

#[inline]
pub fn set_cell_value_as_number(sheet: &mut Worksheet, row: u32, col: u32, value: f64) {
    sheet.cell_mut((col, row)).set_value_number(value);
}

#[inline]
pub fn set_cell_value_bold<S: Into<String>>(sheet: &mut Worksheet, row: u32, col: u32, value: S) {
    let mut text_element = TextElement::default();
    text_element.set_text(value).font_mut().set_bold(true);
    let mut rich_text = RichText::default();
    rich_text.set_rich_text_elements(vec![text_element]);
    sheet.cell_mut((col, row)).set_rich_text(rich_text);
}

#[inline]
pub fn set_cell_note<S: Into<String>>(sheet: &mut Worksheet, row: u32, col: u32, value: S) {
    // Using comments as xlsx does not seem to support the concept of notes but
    // that's what we want when we move over to Google Sheets
    let mut note = Comment::default();
    note.set_text_string(value);
    let coordinate = note.coordinate_mut();
    coordinate.set_col_num(col);
    coordinate.set_row_num(row);
    sheet.add_comments(note);
}

#[deprecated(since = "0.1.3", note = "Use set_auto_size_cols()")]
pub fn set_auto_size<'a>(sheet: &mut Worksheet, cols: impl Iterator<Item = &'a u32>) {
    set_auto_size_cols(sheet, cols);
}

pub fn set_auto_size_cols<'a>(sheet: &mut Worksheet, cols: impl Iterator<Item = &'a u32>) {
    for &col in cols {
        sheet
            .column_dimension_by_number_mut(col)
            .set_auto_width(true);
    }
}

pub fn set_best_fit_cols<'a>(sheet: &mut Worksheet, cols: impl Iterator<Item = &'a u32>) {
    for &col in cols {
        sheet.column_dimension_by_number_mut(col).set_best_fit(true);
    }
}

/// Used to set the size of a set of columns. Expects column index paired with
/// size in an iterator of tuples
pub fn set_size_cols<'a>(
    sheet: &mut Worksheet,
    cols_and_sizes: impl Iterator<Item = &'a (u32, f64)>,
) {
    for &(col, value) in cols_and_sizes {
        sheet.column_dimension_by_number_mut(col).set_width(value);
    }
}

/// Sets the any split values set to `Some(_)` and returns an error if neither
/// are set
pub fn set_frozen_pane(
    sheet: &mut Worksheet,
    vertical_split_value: Option<f64>,
    horizontal_split_value: Option<f64>,
) -> anyhow::Result<()> {
    if vertical_split_value.is_none() && horizontal_split_value.is_none() {
        bail!("neither vertical nor horizontal spit are set");
    }

    let sheet_views = sheet.sheet_views_mut();

    // If no sheet view exists yet, push a default one onto the list
    if sheet_views.sheet_view_list_mut().is_empty() {
        sheet_views.add_sheet_view_list_mut(umya_spreadsheet::SheetView::default());
    }

    let sheet_view = sheet_views
        .sheet_view_list_mut()
        .get_mut(0)
        .context("failed to get first sheet view")?;

    // Initialize the pane if it is currently None
    if sheet_view.pane().is_none() {
        sheet_view.set_pane(umya_spreadsheet::structs::Pane::default());
    }

    let pane = sheet_view
        .pane_mut()
        .context("failed to get pane in sheet_view")?;
    if let Some(value) = vertical_split_value {
        pane.set_vertical_split(value);
    }
    if let Some(value) = horizontal_split_value {
        pane.set_horizontal_split(value);
    }
    pane.set_state(umya_spreadsheet::PaneStateValues::Frozen);
    Ok(())
}

/// If using from a const context see `const_only_alpha_to_index`
pub const fn alpha_to_index(col: &str) -> Option<u32> {
    let bytes = col.as_bytes();
    let mut num = 0;
    let mut i = 0;
    if bytes.is_empty() {
        return None;
    }
    while i < bytes.len() {
        let byte = bytes[i];
        let val = if byte >= b'A' && byte <= b'Z' {
            (byte - b'A' + 1) as u32
        } else if byte >= b'a' && byte <= b'z' {
            (byte - b'a' + 1) as u32
        } else {
            return None; // Invalid character
        };
        num = num * 26 + val;
        i += 1;
    }
    Some(num)
}

/// Const version that panics on invalid input so only intended to be used in
/// const context, use `alpha_to_index` if not being used in a const context
pub const fn const_only_alpha_to_index(col: &str) -> u32 {
    match alpha_to_index(col) {
        Some(x) => x,
        None => panic!("Invalid Column Letter"),
    }
}
