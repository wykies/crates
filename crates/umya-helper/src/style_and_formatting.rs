use anyhow::{Context as _, bail};
use umya_spreadsheet::{Worksheet, helper::coordinate::CellCoordinates};

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

/// See the constants attached to umya-spreadsheet::NumberingFormat eg.
/// umya_spreadsheet::NumberingFormat::FORMAT_CURRENCY_USD_SIMPLE
pub fn set_range_format_to<R: AsRef<str>, F: Into<String>>(
    sheet: &mut Worksheet,
    range: R,
    number_format: F,
) {
    let mut style = umya_spreadsheet::Style::default();
    style.number_format_mut().set_format_code(number_format);
    sheet.set_style_by_range(range.as_ref(), &style);
}

pub fn set_cell_horizontal_alignment<C: Into<CellCoordinates>>(
    sheet: &mut Worksheet,
    coordinate: C,
    alignment: umya_spreadsheet::HorizontalAlignmentValues,
) {
    sheet
        .style_mut(coordinate)
        .alignment_mut()
        .set_horizontal(alignment);
}

pub fn set_manual_page_break_on_row(sheet: &mut Worksheet, row: u32) {
    let mut page_break = umya_spreadsheet::Break::default();
    page_break.set_id(row);
    page_break.set_manual_page_break(true);

    sheet.row_breaks_mut().break_list_mut().push(page_break);
}
