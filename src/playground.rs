use reqwest::Client;
use serde::Deserialize;
use serde::Serialize;

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
    success: bool,
    stdout: String,
    stderr: String,
}

impl PlaygroundResponse {
    pub fn output(self) -> String {
        self.stderr + &self.stdout
    }
}

const PLAYGROUND_RUN_URL: &str = "https://play.rust-lang.org/execute";

pub async fn run_code(client: &Client, code: String) -> anyhow::Result<PlaygroundResponse> {
    fn transform_code(input: String) -> String {
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

    client
        .post(PLAYGROUND_RUN_URL)
        .json(&PlaygroundBody {
            code: transform_code(code),
            channel: "nightly".into(),
            mode: "debug".into(),
            edition: "2021".into(),
            crate_type: "bin".into(),
            tests: false,
            backtrace: false,
        })
        .send()
        .await?
        .error_for_status()?
        .json()
        .await
        .map_err(Into::into)
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

    client
        .post(PLAYGROUND_RUN_URL)
        .json(&PlaygroundBody {
            code: transform_code(test1, test2),
            channel: "nightly".into(),
            mode: "release".into(),
            edition: "2021".into(),
            crate_type: "bin".into(),
            tests: false,
            backtrace: false,
        })
        .send()
        .await?
        .error_for_status()?
        .json()
        .await
        .map_err(Into::into)
}
