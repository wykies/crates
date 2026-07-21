use umya_spreadsheet::Worksheet;

pub fn get_next_empty_row(sheet: &Worksheet, start_row: u32, column_to_check: u32) -> u32 {
    (start_row..)
        .find(|&row| {
            sheet
                .cell((column_to_check, row))
                .is_none_or(|x| x.value().trim().is_empty())
        })
        .expect("runs on an infinite iterator and can only exit with a value")
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
