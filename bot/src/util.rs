use once_cell::sync::Lazy;
use poise::async_trait;
use poise::serenity_prelude::Context;
use poise::serenity_prelude::Message;
use poise::CodeBlock;
use poise::PopArgument;
use regex::Regex;
use std::borrow::Cow;
use std::error::Error;
use sublime_fuzzy::best_match;

pub fn shrink_to_fit(input: &str) -> &str {
    let len = 1980.min(input.len());
    &input[..len]
}

pub fn codeblock(input: &str) -> String {
    let input = shrink_to_fit(input);
    format!("```rs\n{input}\n```")
}
pub fn codeblock_with_lang(lang: &str, input: &str) -> String {
    let input = shrink_to_fit(input);
    format!("```{lang}\n{input}\n```")
}

pub fn strip_ansi(input: &str) -> Cow<'_, str> {
    static ANSI_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new("\\x1b\\[[0-9;]*m").unwrap());
    ANSI_REGEX.replace_all(input, "")
}

pub fn strip_header_stderr(input: &str) -> &str {
    let input = input.trim_start_matches("   Compiling playground v0.0.1 (/playground)");
    let input = input.trim_start();

    // if the next section starts with 'Finished', it implies that we can safely strip away
    // the next 2 lines
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
    } else {
        // Error, warning or ICE
        input
    }
}

pub fn get_worker_path() -> &'static str {
    #[cfg(debug_assertions)]
    {
        "./target/debug/worker"
    }
    #[cfg(not(debug_assertions))]
    {
        "./target/release/worker"
    }
}

pub fn get_temp() -> anyhow::Result<Option<f64>> {
    #[cfg(target_arch = "aarch64")]
    {
        use anyhow::ensure;
        use anyhow::Context;
        use std::process::Command;
        use std::process::Output;

        let Output { status, stdout, .. } = Command::new("/usr/bin/vcgencmd")
            .arg("measure_temp")
            .output()?;

        ensure!(status.success());

        tracing::debug!(?stdout, "Parsing utf-8");

        let output = String::from_utf8(stdout).context("vcgencmd returned invalid utf-8")?;

        let (_, num) = output
            .trim_end()
            .trim_end_matches("'C")
            .split_once('=')
            .with_context(|| {
                tracing::error!(%output, "Wrong format");
                "vcgencmd returned wrong format"
            })?;

        Ok(Some(num.parse()?))
    }
    #[cfg(not(target_arch = "aarch64"))]
    {
        Ok(None)
    }
}

pub struct CodeBlockOrRest {
    pub code: String,
    pub language: Option<String>,
}

#[async_trait]
impl<'a> PopArgument<'a> for CodeBlockOrRest {
    async fn pop_from(
        args: &'a str,
        attachment_index: usize,
        cx: &Context,
        msg: &Message,
    ) -> Result<(&'a str, usize, Self), (Box<dyn Error + Send + Sync + 'static>, Option<String>)>
    {
        if let Ok((rest, index, CodeBlock { code, language })) =
            CodeBlock::pop_from(args, attachment_index, cx, msg).await
        {
            return Ok((rest, index, CodeBlockOrRest { code, language }));
        }

        Ok((
            "",
            attachment_index,
            CodeBlockOrRest {
                code: args.into(),
                language: None,
            },
        ))
    }
}

/// Tests for equality and returns a score.
pub fn fuzzy_match(left: &str, right: &str) -> Option<isize> {
    if left == right {
        // Prefer exact matches. Use some high value that will "probably" be higher than almost-exact matches.
        return Some(10000);
    }

    Some(best_match(left, right)?.score())
}
