use ctxt::CrateCtxt;
use ctxt::DocCtxt;
use ctxt::Output;
use io::Write;
use rayon::prelude::IntoParallelIterator;
use rayon::prelude::ParallelBridge;
use rayon::prelude::ParallelIterator;
use rustdoc_types::Crate;
use rustdoc_types::Function;
use rustdoc_types::ItemEnum;
use rustdoc_types::ItemKind;
use std::env;
use std::fs;
use std::io;
use tracing::debug;
use tracing::info;

macro_rules! prompt {
    ($($t:tt)*) => {{
        print!($($t)*);
        io::stdout().flush()?;
        let mut s = String::new();
        io::stdin().read_line(&mut s)?;
        s
    }};
}

pub mod ctxt;

#[tracing::instrument(skip(ctxt, out))]
pub fn process_crate(DocCtxt { ctxt, out }: &mut DocCtxt) -> anyhow::Result<()> {
    info!(?ctxt.krate.root, "processing crate");
    ctxt.run_visitor(out);
    info!(?ctxt.krate.root, "finished crate");
    println!("{} {}", out.index.len(), out.docs.len());
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
            process_crate(&mut ctx)
        })
        .collect::<Vec<_>>();
    // .collect::<Vec<_>>();

    // let ctx = DocCtxt::new();

    // for path in paths {
    //     process_json(&ctx, &path);
    // }
    // loop {
    //     let resp = prompt!("Using paths: {:?} [y/n] ", paths);
    //     match resp.trim() {
    //         "y" | "yes" => break,
    //         "n" | "no" => return Ok(()),
    //         _ => println!("Please enter 'y' or 'n'."),
    //     }
    // }

    Ok(())
}
