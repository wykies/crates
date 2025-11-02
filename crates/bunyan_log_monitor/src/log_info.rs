use std::{
    fmt::Display,
    fs,
    io::{self, BufRead as _},
    ops::ControlFlow,
    path::Path,
    sync::OnceLock,
};

use anyhow::{anyhow, bail, Context};
use chrono::NaiveDateTime;
use regex::Regex;

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct LogInfo {
    pub date_time: NaiveDateTime,
    pub abnormal_outcome: Option<String>,
    pub errors_and_warnings: Vec<String>,
}

impl LogInfo {
    pub fn new<S: AsRef<str>>(file_name: S) -> anyhow::Result<Self> {
        static CELL_RE: OnceLock<Regex> = OnceLock::new();
        let re = CELL_RE.get_or_init(|| {
            Regex::new(r"(\d\d\d\d-\d\d-\d\d \d\d\d\d\d\d)\.\d\d\d ?(\[.+\])?\.html")
                .expect("failed to compile regex")
        });

        let Some(captures) = re.captures(file_name.as_ref()) else {
            // Assumption: Only log files are present in the log folder
            bail!("regex failed to match filename: {}", file_name.as_ref())
        };
        // Regex matched and can only match if first capture group is found as it is not optional
        let date_time_str = captures.get(1).expect("required for match").as_str();
        let abnormal_outcome = captures.get(2).map(|x| x.as_str().to_string());
        let date_time = NaiveDateTime::parse_from_str(date_time_str, "%F %H%M%S")
            .with_context(|| format!("failed to parse date from {date_time_str:?}"))?;
        Ok(Self {
            date_time,
            abnormal_outcome,
            errors_and_warnings: Default::default(),
        })
    }

    /// Expects to receive the input without the surrounding tags but including inner tags to be replaced
    fn add_error_or_warning(&mut self, msg: String) {
        let msg = msg.replace("&quot;", "\"");
        let msg = msg.replace("<br>", "; ");
        self.errors_and_warnings.push(msg);
    }

    pub fn extract_errors(&mut self, file_path: &Path) -> anyhow::Result<()> {
        let mut extract_state = ExtractState::FindTableStart;
        for line in read_lines(file_path)? {
            let line = line.with_context(|| format!("failed to read line in {:?}", file_path))?;
            extract_state = match extract_state {
                ExtractState::FindTableStart => find_start_of_table(&line),
                ExtractState::FindMsg => match self.check_for_start_of_msg(line) {
                    ControlFlow::Continue(value) => value,
                    ControlFlow::Break(value) => return value,
                },
                ExtractState::ReadingMsg(count, partial_msg) => {
                    self.continue_reading_msg(count, partial_msg, line)?
                }
            }
        }
        bail!("unexpected end of file")
    }

    fn check_for_start_of_msg(
        &mut self,
        line: String,
    ) -> ControlFlow<anyhow::Result<()>, ExtractState> {
        let line_trimmed = line.trim_start();
        if !line_trimmed.starts_with("<td>") {
            // Not the start of the message check for end of table or go to next line
            if line_trimmed.starts_with("</table>") {
                ControlFlow::Break(Ok(()))
            } else {
                // Go to next line
                ControlFlow::Continue(ExtractState::FindMsg)
            }
        } else {
            // Check for case of single line
            static CELL_RE_SINGLE_LINE: OnceLock<Regex> = OnceLock::new();
            let re = CELL_RE_SINGLE_LINE
                .get_or_init(|| Regex::new(r"<td>(.+)<\/td>").expect("failed to compile regex"));

            if let Some(captures) = re.captures(&line) {
                // Single line found
                self.add_error_or_warning(
                    captures
                        .get(1)
                        .expect("required for match")
                        .as_str()
                        .to_string(),
                );
                ControlFlow::Continue(ExtractState::FindMsg) // Look for next msg
            } else {
                // First part of a multiline message
                static CELL_RE_START_OF_MSG: OnceLock<Regex> = OnceLock::new();
                let re = CELL_RE_START_OF_MSG
                    .get_or_init(|| Regex::new(r"<td>(.+)").expect("failed to compile regex"));
                if let Some(captures) = re.captures(&line) {
                    // Single line found
                    let first_part_of_msg = captures
                        .get(1)
                        .expect("required for match")
                        .as_str()
                        .to_string();
                    ControlFlow::Continue(ExtractState::ReadingMsg(1, first_part_of_msg))
                } else {
                    ControlFlow::Break(Err(anyhow!("something wrong with the logic, we checked that this line starts a message but then... it doesn't now?")))
                }
            }
        }
    }

    fn continue_reading_msg(
        &mut self,
        count: u8,
        mut partial_msg: String,
        line: String,
    ) -> anyhow::Result<ExtractState> {
        static CELL_RE_LAST_LINE: OnceLock<Regex> = OnceLock::new();
        let re = CELL_RE_LAST_LINE
            .get_or_init(|| Regex::new(r"(.*)<\/td>").expect("failed to compile regex"));

        if let Some(captures) = re.captures(&line) {
            partial_msg.push_str(
                &captures
                    .get(1)
                    .map(|x| x.as_str().to_string())
                    .unwrap_or_default(),
            );
            self.add_error_or_warning(partial_msg);
            Ok(ExtractState::FindMsg)
        } else {
            // Not last line add value and keep reading
            if count >= ExtractState::MAX_MSG_LINES {
                bail!("something is wrong found too many lines in msg. Past {} which was with line: {line:?}", ExtractState::MAX_MSG_LINES)
            }
            partial_msg.push_str(&line);
            Ok(ExtractState::ReadingMsg(count + 1, partial_msg))
        }
    }
}

fn find_start_of_table(line: &str) -> ExtractState {
    static CELL_RE_TABLE_START: OnceLock<Regex> = OnceLock::new();
    let re = CELL_RE_TABLE_START.get_or_init(|| {
        Regex::new(r"<div.*Errors and warnings:").expect("failed to compile regex")
    });
    if re.is_match(line) {
        ExtractState::FindMsg
    } else {
        ExtractState::FindTableStart
    }
}

impl Display for LogInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} {} errors: {}\n{}\n",
            self.abnormal_outcome.as_deref().unwrap_or("[ - ]"),
            self.date_time.format("%F %H:%M:%S"),
            self.errors_and_warnings.len(),
            self.errors_and_warnings.join("\n")
        )
    }
}

// The output is wrapped in a Result to allow matching on errors
// Returns an Iterator to the Reader of the lines of the file.
fn read_lines<P: AsRef<Path>>(path: P) -> anyhow::Result<io::Lines<io::BufReader<fs::File>>> {
    let file = fs::File::open(&path)
        .with_context(|| format!("failed to open file: {:?}", path.as_ref()))?;
    Ok(io::BufReader::new(file).lines())
}

enum ExtractState {
    FindTableStart,
    FindMsg,
    ReadingMsg(u8, String),
}

impl ExtractState {
    /// Detect error if message is more than 5 lines long. Actually expecting exactly 2.
    const MAX_MSG_LINES: u8 = 5;
}
