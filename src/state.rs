use reqwest::Client;

use crate::godbolt;
use crate::playground;

#[derive(Debug)]
pub struct State {
    pub reqwest: Client,
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

    pub async fn get_asm(&self, code: String) -> anyhow::Result<godbolt::GodboltResponse> {
        godbolt::get_asm(&self.reqwest, code).await
    }
}
