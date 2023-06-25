use std::time::Duration;

use anyhow::bail;
use anyhow::Context;
use human_size::Byte;
use human_size::Megabyte;
use human_size::SpecificSize;
use ipc2_host::workerset::TimeoutAction;
use itertools::Itertools;
use poise::samples::HelpConfiguration;
use poise::CodeBlock;
use shared::ClientMessage;
use shared::HostMessage;
use std::fmt::Write;
use sublime_fuzzy::best_match;
use sysinfo::CpuExt;
use sysinfo::SystemExt;

use crate::godbolt;
use crate::godbolt::languages::Rust;
use crate::godbolt::languages::C;
use crate::godbolt::GodboltResponse;
use crate::playground;
use crate::state::State;
use crate::util;
use crate::util::CodeBlockOrRest;
use crate::util::MaybeQuoted;
use crate::PoiseContext;

async fn reply(cx: &PoiseContext<'_>, content: String) -> Result<(), serenity::Error> {
    cx.send(|reply| reply.allowed_mentions(|m| m.empty_parse()).content(content))
        .await?;
    Ok(())
}

/// Executes a Rust codeblock
///
/// The code can simply be an expression and the bot will automatically
/// wrap it in a main function and a print statement.
#[poise::command(prefix_command, track_edits, broadcast_typing)]
pub async fn rust(cx: PoiseContext<'_>, block: CodeBlockOrRest) -> anyhow::Result<()> {
    let response = playground::run_code(&cx.data().reqwest, block.code).await?;

    reply(
        &cx,
        util::codeblock(util::strip_header_stderr(&response.output())),
    )
    .await?;

    Ok(())
}

/// Benchmarks two Rust codeblocks to see which one runs faster
#[poise::command(prefix_command, track_edits, broadcast_typing)]
pub async fn bench(
    cx: PoiseContext<'_>,
    block1: CodeBlock,
    block2: CodeBlock,
) -> anyhow::Result<()> {
    let response = playground::bench_code(&cx.data().reqwest, block1.code, block2.code).await?;

    reply(
        &cx,
        util::codeblock(util::strip_header_stderr(&response.output())),
    )
    .await?;

    Ok(())
}

/// Runs a codeblock under miri, an interpreter that checks for memory errors
#[poise::command(prefix_command, track_edits, broadcast_typing)]
pub async fn miri(cx: PoiseContext<'_>, block: CodeBlockOrRest) -> anyhow::Result<()> {
    let response = playground::run_miri(&cx.data().reqwest, block.code).await?;
    reply(
        &cx,
        util::codeblock(util::strip_header_stderr(&response.output())),
    )
    .await?;

    Ok(())
}

/// Runs a codeblock under clippy, a Rust linter
#[poise::command(prefix_command, track_edits, broadcast_typing)]
pub async fn clippy(cx: PoiseContext<'_>, block: CodeBlockOrRest) -> anyhow::Result<()> {
    let response = playground::run_clippy(&cx.data().reqwest, block.code).await?;
    reply(&cx, util::codeblock(&response.output())).await?;

    Ok(())
}

/// Runs a codeblock under clippy, a Rust linter
#[poise::command(prefix_command, track_edits, broadcast_typing)]
pub async fn expand(cx: PoiseContext<'_>, block: CodeBlockOrRest) -> anyhow::Result<()> {
    let response = playground::run_macro_expansion(&cx.data().reqwest, block.code).await?;
    reply(&cx, util::codeblock(&response.output())).await?;

    Ok(())
}

/// Runs a codeblock under clippy, a Rust linter
#[poise::command(prefix_command, track_edits, broadcast_typing)]
pub async fn godbolt(
    cx: PoiseContext<'_>,
    flags: Option<MaybeQuoted>,
    block: CodeBlockOrRest,
) -> anyhow::Result<()> {
    let response =
        compile_any_lang(&cx.data().reqwest, block.into(), flags.map(|q| q.value)).await?;
    reply(&cx, util::codeblock(&response.0)).await?;

    Ok(())
}

/// Help me
#[poise::command(prefix_command, track_edits)]
pub async fn help(cx: PoiseContext<'_>, command: Option<String>) -> anyhow::Result<()> {
    let config = HelpConfiguration {
        extra_text_at_bottom:
            "You can edit your message to the bot and the bot will edit its response",
        ..Default::default()
    };

    poise::builtins::help(cx, command.as_deref(), config).await?;
    Ok(())
}

async fn compile_any_lang(
    reqwest: &reqwest::Client,
    CodeBlock { code, language }: CodeBlock,
    flags: Option<String>,
) -> anyhow::Result<GodboltResponse> {
    Ok(match language.as_deref() {
        Some("rs" | "rust") | None => godbolt::get_asm::<Rust>(reqwest, code, flags).await?,
        Some("c") => godbolt::get_asm::<C>(reqwest, code, flags).await?,
        Some(other) => bail!("unknown codeblock language: {other}"),
    })
}

/// Compile a codeblock and get the assembly
#[poise::command(prefix_command, track_edits)]
pub async fn asm(cx: PoiseContext<'_>, blocks: Vec<CodeBlock>) -> anyhow::Result<()> {
    let mut output = String::new();
    let reqwest = &cx.data().reqwest;

    for block in blocks {
        let out = compile_any_lang(reqwest, block, None).await?;
        output.push_str(&util::codeblock(&out.0));
    }

    reply(&cx, output).await?;
    Ok(())
}

/// Compile two codeblocks and diff them
#[poise::command(prefix_command, track_edits)]
pub async fn asmdiff(
    cx: PoiseContext<'_>,
    block1: CodeBlock,
    block2: CodeBlock,
) -> anyhow::Result<()> {
    let State { reqwest, .. } = &**cx.data();
    let response1 = compile_any_lang(reqwest, block1, None).await?;
    let response2 = compile_any_lang(reqwest, block2, None).await?;

    reply(
        &cx,
        util::codeblock_with_lang("diff", &response1.diff(response2)),
    )
    .await?;
    Ok(())
}

const MAX_TIME: Duration = Duration::from_secs(5);

/// Executes JavaScript code
#[poise::command(prefix_command, track_edits)]
pub async fn js(cx: PoiseContext<'_>, block: CodeBlockOrRest) -> anyhow::Result<()> {
    let CodeBlockOrRest { code, .. } = block;
    tracing::info!(%code, "Send JS code to worker");

    let ClientMessage::EvalResponse(message) = cx
        .data()
        .workers
        .send_timeout(HostMessage::Eval(code), MAX_TIME, TimeoutAction::Restart)
        .await?;

    reply(
        &cx,
        util::codeblock_with_lang(
            "js",
            match &message {
                Ok(x) => x,
                Err(x) => x,
            },
        ),
    )
    .await?;

    Ok(())
}

#[poise::command(prefix_command, track_edits)]
pub async fn info(cx: PoiseContext<'_>) -> anyhow::Result<()> {
    let output = {
        let temperature = util::get_temp()?
            .map(|t| t.to_string())
            .unwrap_or_else(|| "<unsupported>".into());

        let mut sys = cx.data().system.lock().unwrap();
        sys.refresh_all();

        let mut output = format!("```\nTemperature: {temperature}\n");

        for (id, cpu) in sys.cpus().iter().enumerate() {
            let _ = writeln!(output, "CPU #{id}: {:.2}%", cpu.cpu_usage());
        }

        let fmt_size = |bytes| {
            SpecificSize::new(bytes as f64, Byte)
                .unwrap()
                .into::<Megabyte>()
        };

        let total = fmt_size(sys.total_memory());
        let avail = fmt_size(sys.free_memory());
        let ratio = (avail.to_bytes() as f64 / total.to_bytes() as f64) * 100.0;

        let _ = writeln!(
            output,
            "Memory: {:.2}/{:.2} ({:.2}%)\n```",
            avail, total, ratio
        );

        output
    };

    reply(&cx, output).await?;

    Ok(())
}

#[poise::command(prefix_command, track_edits)]
pub async fn fuzzy(cx: PoiseContext<'_>, query: String, search: String) -> anyhow::Result<()> {
    let result = best_match(&query, &search).context("No match!")?;

    let message = format!(
        "Score: {} \n\
        Matched indices: {}
    ",
        result.score(),
        result.matched_indices().join(", ")
    );

    reply(&cx, message).await?;
    Ok(())
}

#[poise::command(prefix_command, track_edits)]
pub async fn find(cx: PoiseContext<'_>, query: String) -> anyhow::Result<()> {
    let items = cx.data().docs.find(&query).take(10);
    let msg = items.fold(String::new(), |mut prev, (score, path, _)| {
        let _ = write!(prev, "{}: {}\n", score, path);
        prev
    });
    reply(&cx, msg).await?;
    Ok(())
}

#[poise::command(prefix_command, track_edits)]
pub async fn docs(cx: PoiseContext<'_>, query: String) -> anyhow::Result<()> {
    let docs = &cx.data().docs;
    let (_, _, docs) = docs.find(&query).next().context("Nothing found!")?;

    reply(&cx, docs.into()).await?;
    Ok(())
}
