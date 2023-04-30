use std::sync::Mutex;

use anyhow::Context;
use ipc2_host::workerset::WorkerSet;
use itertools::Itertools;
use reqwest::Client;
use shared::ClientMessage;
use shared::HostMessage;
use sysinfo::CpuRefreshKind;
use sysinfo::RefreshKind;
use sysinfo::System;
use sysinfo::SystemExt;
use tokio::fs;
use tokio::net::UnixListener;

use crate::util;

pub struct Docs {
    index: Vec<Box<str>>,
    docs: Vec<Box<str>>,
}
impl Docs {
    async fn from_path(path: &str) -> anyhow::Result<Self> {
        let bin = fs::read(path)
            .await
            .context("Could not open docs binary blob. Make sure to run the doc-converter on your doc .json files.")?;
        let (index, docs) =
            bincode::deserialize(&bin).context("Failed to deserialize docs binary blob")?;
        Ok(Self { index, docs })
    }

    pub fn find(&self, query: &str) -> impl Iterator<Item = (isize, &str, &str)> {
        let query_last_segment = query.rsplit("::").next();
        self.index
            .iter()
            .zip(&self.docs)
            .map(|(path, docs)| {
                let mut score = util::fuzzy_match(query, path).unwrap_or_default();
                let path_last_segment = path.rsplit("::").next();
                if let Some((query, path)) = query_last_segment.zip(path_last_segment) {
                    if query == path {
                        score += 1000;
                    }
                }

                (score, path.as_ref(), docs.as_ref())
            })
            .sorted_by(|a, b| b.0.cmp(&a.0))
    }
}

pub struct State {
    pub workers: WorkerSet<UnixListener, ClientMessage, HostMessage>,
    pub reqwest: Client,
    pub system: Mutex<System>,
    pub docs: Docs,
}

impl State {
    pub async fn new() -> anyhow::Result<Self> {
        let path = util::get_worker_path();
        tracing::info!(%path, "Creating state");
        let docs = Docs::from_path("doc.bin").await?;

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
