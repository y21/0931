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
