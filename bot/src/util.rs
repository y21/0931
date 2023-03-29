use once_cell::sync::Lazy;
use regex::Regex;
use std::borrow::Cow;

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
