use std::env;

use anyhow::Result;

pub(crate) mod analysis;
pub(crate) mod item;

fn shorten_text(text: &str) -> Result<String> {
    let min_line_len: usize = env::var("JOB_TEXT_MIN_LINE_LEN").unwrap_or("80".to_string()).parse()?;
    let max_total_len: usize = env::var("JOB_TEXT_MAX_TOTAL_LEN")
        .unwrap_or("4800".to_string())
        .parse()?;
    let mut lines = vec![];
    let mut total_len = 0;
    for line in text
        .split("\n")
        .into_iter()
        .map(|l| format!("- {}", l.trim()))
        .collect::<Vec<String>>()
    {
        let length = line.len();
        if total_len + length > max_total_len {
            continue;
        }
        if min_line_len <= length {
            lines.push(line);
            total_len += length;
        }
    }
    return Ok(lines.join("\n"));
}
