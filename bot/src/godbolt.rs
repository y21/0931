use reqwest::Client;
use serde::Deserialize;
use std::fmt::Write;

use crate::util;

use self::languages::CompileTarget;

#[derive(Deserialize)]
pub struct GodboltAsmBlock {
    pub text: String,
}

#[derive(Deserialize, Debug)]
pub struct GodboltResponse(pub String);

impl GodboltResponse {
    pub fn diff(self, other: Self) -> String {
        let Self(this) = self;
        let Self(other) = other;
        let mut output = String::new();

        for diff in diff::lines(this.trim_end(), other.trim_end()) {
            let _ = match diff {
                diff::Result::Left(l) => writeln!(output, "- {l}"),
                diff::Result::Both(l, _) => writeln!(output, "  {l}"),
                diff::Result::Right(r) => writeln!(output, "+ {r}"),
            };
        }

        output
    }
}
pub mod languages {
    use serde_json::json;

    pub trait CompileTarget {
        fn url() -> &'static str;
        fn prepare_json_body(source: &str, flags: Option<&str>) -> serde_json::Value;
    }

    pub struct Rust;
    impl CompileTarget for Rust {
        fn url() -> &'static str {
            "https://godbolt.org/api/compiler/nightly/compile"
        }
        fn prepare_json_body(source: &str, flags: Option<&str>) -> serde_json::Value {
            json!({
                "source": source,
                "compiler": "nightly",
                "options": {
                    "userArguments": flags.unwrap_or("-Copt-level=3 -Clto=on -Ctarget-feature=+sse3,+avx -Ctarget-cpu=native")
                },
                "lang": "rust",
                "allowStoreCodeDebug": true
            })
        }
    }

    pub struct C;
    impl CompileTarget for C {
        fn url() -> &'static str {
            "https://godbolt.org/api/compiler/cclang1600/compile"
        }
        fn prepare_json_body(source: &str, flags: Option<&str>) -> serde_json::Value {
            json!({
                "source": source,
                "compiler": "cclang1600",
                "options": {
                    "userArguments": flags.unwrap_or("-O3 -march=native")
                },
                "lang": "c",
                "allowStoreCodeDebug": true
            })
        }
    }
}

pub async fn get_asm<T: CompileTarget>(
    client: &Client,
    input: String,
    flags: Option<String>,
) -> anyhow::Result<GodboltResponse> {
    let response = client
        .post(T::url())
        .json(&T::prepare_json_body(&input, flags.as_deref()))
        .send()
        .await?
        .error_for_status()?
        .text()
        .await?;

    let mut response = util::strip_ansi(&response).into_owned();
    assert!(response.starts_with("# Compilation")); // if this fails, the API changed and we need to fix this either way
    response.drain(..response.find('\n').unwrap() + 1);

    Ok(GodboltResponse(response))
}
