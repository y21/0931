#![feature(let_chains)]

use anyhow::Context;
use ctxt::CrateCtxt;
use ctxt::DocCtxt;
use ctxt::Output;
use rayon::prelude::IntoParallelIterator;
use rayon::prelude::ParallelIterator;
use rustdoc_types::Crate;
use std::env;
use std::fs;
use tracing::info;

pub mod ctxt;

#[tracing::instrument(skip(ctxt, out))]
pub fn process_crate(DocCtxt { ctxt, out }: &mut DocCtxt) -> anyhow::Result<()> {
    info!(?ctxt.krate.root, "processing crate");
    ctxt.run_visitor(out);
    info!(?ctxt.krate.root, "finished crate");
    Ok(())
}

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let paths = env::args().skip(1).collect::<Vec<_>>();
    let crates = paths
        .into_par_iter()
        .map(|path| -> anyhow::Result<_> {
            let contents = fs::read_to_string(path)?;
            let krate: Crate = serde_json::from_str(&contents)?;
            let mut ctx = DocCtxt {
                ctxt: CrateCtxt { krate },
                out: Output::default(),
            };
            process_crate(&mut ctx)?;
            Ok(ctx.out)
        })
        .collect::<anyhow::Result<Vec<_>>>()?;

    // merge outputs
    let output = crates
        .into_iter()
        .reduce(|mut prev, cur| {
            prev.index.extend(cur.index);
            prev.docs.extend(cur.docs);
            prev
        })
        .unwrap_or_default();

    info!("documented {} items", output.index.len());
    let bin =
        bincode::serialize(&(output.index, output.docs)).context("Failed to serialize output")?;

    fs::write("doc.bin", bin).context("Failed to write output")?;

    Ok(())
}
