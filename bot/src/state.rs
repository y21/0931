use ipc2_host::workerset::WorkerSet;
use reqwest::Client;
use shared::ClientMessage;
use shared::HostMessage;
use tokio::net::UnixListener;

use crate::godbolt;
use crate::playground;
use crate::util;

pub struct State {
    pub workers: WorkerSet<UnixListener, ClientMessage, HostMessage>,
    pub reqwest: Client,
}

impl State {
    pub async fn new() -> anyhow::Result<Self> {
        let path = util::get_worker_path();
        tracing::info!(%path, "Creating state");

        Ok(Self {
            workers: WorkerSet::builder().worker_path(path).finish().await?,
            reqwest: Client::new(),
        })
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
