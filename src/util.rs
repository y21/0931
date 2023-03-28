use std::borrow::Cow;

use once_cell::sync::Lazy;
use regex::Regex;

pub fn codeblock(input: &str) -> String {
    format!("```rs\n{input}\n```")
}
pub fn codeblock_with_lang(lang: &str, input: &str) -> String {
    format!("```{lang}\n{input}\n```")
}

pub fn strip_ansi(input: &str) -> Cow<'_, str> {
    static ANSI_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new("\\x1b\\[[0-9;]*m").unwrap());
    ANSI_REGEX.replace_all(input, "")
}

pub fn strip_header_stderr(input: &str) -> &str {
    let input = input.trim_start_matches("   Compiling playground v0.0.1 (/playground)");
    let input = input.trim_start();
    if input.starts_with("Finished") {
        let mut newlines = 0;
        input.trim_start_matches(move |c| {
            if c == '\n' {
                newlines += 1;
                if newlines == 2 {
                    return false;
                }
            }
            true
        })
    } else if input.starts_with("error") || input.starts_with("warning:") {
        input
    } else {
        unreachable!("{input}")
    }
}
