use jiff::civil;
use umya_spreadsheet::{
    Comment, RichText, Style, TextElement, Worksheet, helper::date::jiff_date_time_to_excel,
};

#[inline]
pub fn set_cell_value<S: Into<String>>(sheet: &mut Worksheet, row: u32, col: u32, value: S) {
    sheet.cell_mut((col, row)).set_value(value);
}

#[inline]
pub fn set_cell_value_as_number(sheet: &mut Worksheet, row: u32, col: u32, value: f64) {
    sheet.cell_mut((col, row)).set_value_number(value);
}

/// This function exists as a convenience because just setting the style on
/// large ranges increases the file size a lot. So this makes it easy to just
/// keep apply the same formula as you write the values. See
/// `set_range_format_to` for how to create a style
#[inline]
pub fn set_cell_value_as_number_with_style(
    sheet: &mut Worksheet,
    row: u32,
    col: u32,
    value: f64,
    style: &Style,
) {
    *sheet
        .cell_mut((col, row))
        .set_value_number(value)
        .style_mut() = style.clone();
}

#[inline]
pub fn set_cell_value_as_datetime(
    sheet: &mut Worksheet,
    row: u32,
    col: u32,
    value: civil::DateTime,
) {
    let value = jiff_date_time_to_excel(value);
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
