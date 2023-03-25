use reqwest::Client;

use crate::playground;

#[derive(Debug)]
pub struct State {
    reqwest: Client,
}

impl State {
    pub fn new() -> Self {
        Self {
            reqwest: Client::new(),
        }
    }

    pub async fn run_code(&self, code: String) -> anyhow::Result<playground::PlaygroundResponse> {
        playground::run_code(&self.reqwest, code).await
    }

    pub async fn bench_code(
        &self,
        test1: String,
        test2: String,
    ) -> anyhow::Result<playground::PlaygroundResponse> {
        playground::bench_code(&self.reqwest, test1, test2).await
    }
}
