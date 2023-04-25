use std::sync::Mutex;

use anyhow::Context;
use ipc2_host::workerset::WorkerSet;
use reqwest::Client;
use shared::ClientMessage;
use shared::HostMessage;
use sysinfo::CpuRefreshKind;
use sysinfo::RefreshKind;
use sysinfo::System;
use sysinfo::SystemExt;
use tokio::fs;
use tokio::net::UnixListener;

use crate::docs::Docs;
use crate::util;

pub struct State {
    pub workers: WorkerSet<UnixListener, ClientMessage, HostMessage>,
    pub reqwest: Client,
    pub system: Mutex<System>,
    pub docs: Docs,
}

impl State {
    pub async fn new() -> anyhow::Result<Self> {
        let read = |path: &'static str| async move {
            fs::read_to_string(path).await.context("Reading JSON file")
        };
        let stellar_json = read("./stellar_canvas.json").await?;
        let std_json = read("./std.json").await?;
        let core_json = read("./core.json").await?;
        let alloc_json = read("./alloc.json").await?;

        let mut docs = Docs::new();
        docs.add_crate_json(&stellar_json)?;
        docs.add_crate_json(&std_json)?;
        docs.add_crate_json(&core_json)?;
        docs.add_crate_json(&alloc_json)?;

        let path = util::get_worker_path();
        tracing::info!(%path, "Creating state");

        Ok(Self {
            workers: WorkerSet::builder().worker_path(path).finish().await?,
            reqwest: Client::new(),
            system: Mutex::new(System::new_with_specifics(
                RefreshKind::new()
                    .with_memory()
                    .with_cpu(CpuRefreshKind::new().with_cpu_usage()),
            )),
            docs,
        })
    }
}
