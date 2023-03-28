use reqwest::Client;
use serde::Deserialize;
use serenity::json::json;
use std::fmt::Write;

use crate::util;

const GODBOLT_COMPILE_URL: &str = "https://godbolt.org/api/compiler/nightly/compile";

#[derive(Deserialize)]
pub struct GodboltAsmBlock {
    pub text: String,
}

#[derive(Deserialize)]
pub struct GodboltResponse(pub String);

impl GodboltResponse {
    pub fn diff(self, other: Self) -> String {
        let Self(this) = self;
        let Self(other) = other;
        let mut output = String::new();

        for diff in diff::lines(&this, &other) {
            let _ = match diff {
                diff::Result::Left(l) => writeln!(output, "- {l}"),
                diff::Result::Both(l, _) => writeln!(output, "  {l}"),
                diff::Result::Right(r) => writeln!(output, "+ {r}"),
            };
        }

        output
    }
}

pub async fn get_asm(client: &Client, input: String) -> anyhow::Result<GodboltResponse> {
    let response = client
        .post(GODBOLT_COMPILE_URL)
        .json(&json!({
            "source": input,
            "compiler": "nightly",
            "options": {
                "userArguments": "-Copt-level=3 -Clto=on -Ctarget-feature=+sse3,+avx -Ctarget-cpu=native"
            },
            "lang": "rust",
            "allowStoreCodeDebug": true
        }))
        .send()
        .await?
        .error_for_status()?
        .text()
        .await?;

    Ok(GodboltResponse(util::strip_ansi(&response).into_owned()))
}
