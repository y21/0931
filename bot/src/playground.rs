use reqwest::Client;
use serde::Deserialize;
use serde::Serialize;
use serenity::json::json;

#[derive(Serialize)]
pub struct PlaygroundBody {
    channel: String,
    mode: String,
    edition: String,
    #[serde(rename = "crateType")]
    crate_type: String,
    tests: bool,
    code: String,
    backtrace: bool,
}

#[derive(Deserialize, Debug)]
pub struct PlaygroundResponse {
    stdout: String,
    stderr: String,
}

impl PlaygroundResponse {
    pub fn output(self) -> String {
        self.stderr + &self.stdout
    }
}

const PLAYGROUND_RUN_URL: &str = "https://play.rust-lang.org/execute";
const MIRI_RUN_URL: &str = "https://play.rust-lang.org/miri";
const CLIPPY_RUN_URL: &str = "https://play.rust-lang.org/clippy";

fn wrap_in_println(input: String) -> String {
    if input.contains("fn main") {
        input
    } else {
        format!(
            r#"
fn main() {{
    println!("{{:?}}", {{
        {input}
    }});
}}
    "#
        )
    }
}

async fn send_raw_playground_request(
    client: &Client,
    body: PlaygroundBody,
) -> anyhow::Result<PlaygroundResponse> {
    client
        .post(PLAYGROUND_RUN_URL)
        .json(&body)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await
        .map_err(Into::into)
}

pub async fn run_code(client: &Client, code: String) -> anyhow::Result<PlaygroundResponse> {
    send_raw_playground_request(
        client,
        PlaygroundBody {
            channel: "nightly".into(),
            mode: "debug".into(),
            edition: "2021".into(),
            crate_type: "bin".into(),
            tests: false,
            code: wrap_in_println(code),
            backtrace: false,
        },
    )
    .await
}

pub async fn bench_code(
    client: &Client,
    test1: String,
    test2: String,
) -> anyhow::Result<PlaygroundResponse> {
    fn transform_code(test1: String, test2: String) -> String {
        const TEMPLATE: &str = include_str!("benchmark_code.rs");
        TEMPLATE
            .replace("/*{{TEST1}}*/", &test1)
            .replace("/*{{TEST2}}*/", &test2)
    }

    send_raw_playground_request(
        client,
        PlaygroundBody {
            code: transform_code(test1, test2),
            channel: "nightly".into(),
            mode: "release".into(),
            edition: "2021".into(),
            crate_type: "bin".into(),
            tests: false,
            backtrace: false,
        },
    )
    .await
}

pub async fn run_miri(client: &Client, code: String) -> anyhow::Result<PlaygroundResponse> {
    client
        .post(MIRI_RUN_URL)
        .json(&json!({
            "code": wrap_in_println(code),
            "edition": "2021",
        }))
        .send()
        .await?
        .error_for_status()?
        .json()
        .await
        .map_err(Into::into)
}

pub async fn run_clippy(client: &Client, code: String) -> anyhow::Result<PlaygroundResponse> {
    client
        .post(CLIPPY_RUN_URL)
        .json(&json!({
            "code": wrap_in_println(code),
            "edition": "2021",
        }))
        .send()
        .await?
        .error_for_status()?
        .json()
        .await
        .map_err(Into::into)
}
