use std::sync::Mutex;

use ipc2_host::workerset::WorkerSet;
use reqwest::Client;
use shared::ClientMessage;
use shared::HostMessage;
use sysinfo::CpuRefreshKind;
use sysinfo::RefreshKind;
use sysinfo::System;
use sysinfo::SystemExt;
use tokio::net::UnixListener;

use crate::godbolt;
use crate::playground;
use crate::util;

pub struct State {
    pub workers: WorkerSet<UnixListener, ClientMessage, HostMessage>,
    pub reqwest: Client,
    pub system: Mutex<System>,
}

impl State {
    pub async fn new() -> anyhow::Result<Self> {
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
        })
    }
}
