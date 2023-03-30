use std::time::Duration;

use human_size::Byte;
use human_size::Megabyte;
use human_size::SpecificSize;
use ipc2_host::workerset::TimeoutAction;
use poise::samples::HelpConfiguration;
use poise::CodeBlock;
use shared::ClientMessage;
use shared::HostMessage;
use std::fmt::Write;
use sysinfo::CpuExt;
use sysinfo::SystemExt;

use crate::godbolt;
use crate::playground;
use crate::state::State;
use crate::util;
use crate::PoiseContext;

/// Executes a Rust codeblock
///
/// The code can simply be an expression and the bot will automatically
/// wrap it in a main function and a print statement.
#[poise::command(prefix_command, track_edits, broadcast_typing, rename = "run")]
pub async fn run_rust(cx: PoiseContext<'_>, block: CodeBlock) -> anyhow::Result<()> {
    let response = playground::run_code(&cx.data().reqwest, block.code).await?;

    cx.say(util::codeblock(util::strip_header_stderr(
        &response.output(),
    )))
    .await?;

    Ok(())
}

/// Benchmarks two Rust codeblocks to see which one runs faster
#[poise::command(prefix_command, track_edits, broadcast_typing, rename = "bench")]
pub async fn run_bench(
    cx: PoiseContext<'_>,
    block1: CodeBlock,
    block2: CodeBlock,
) -> anyhow::Result<()> {
    let response = playground::bench_code(&cx.data().reqwest, block1.code, block2.code).await?;

    cx.say(util::codeblock(util::strip_header_stderr(
        &response.output(),
    )))
    .await?;

    Ok(())
}

/// Runs a codeblock under miri, an interpreter that checks for memory errors
#[poise::command(prefix_command, track_edits, broadcast_typing, rename = "miri")]
pub async fn run_miri(cx: PoiseContext<'_>, block: CodeBlock) -> anyhow::Result<()> {
    let response = playground::run_miri(&cx.data().reqwest, block.code).await?;
    cx.say(util::codeblock(util::strip_header_stderr(
        &response.output(),
    )))
    .await?;

    Ok(())
}

/// Help me
#[poise::command(prefix_command, track_edits, rename = "help")]
pub async fn run_help(cx: PoiseContext<'_>, command: Option<String>) -> anyhow::Result<()> {
    let config = HelpConfiguration {
        extra_text_at_bottom:
            "You can edit your message to the bot and the bot will edit its response",
        ..Default::default()
    };

    poise::builtins::help(cx, command.as_deref(), config).await?;
    Ok(())
}

/// Compile a codeblock and get the assembly
#[poise::command(prefix_command, track_edits, rename = "asm")]
pub async fn run_asm(cx: PoiseContext<'_>, blocks: Vec<CodeBlock>) -> anyhow::Result<()> {
    let mut output = String::new();

    for block in blocks {
        let out = godbolt::get_asm(&cx.data().reqwest, block.code).await?;
        output.push_str(&util::codeblock(&out.0));
    }

    cx.say(&output).await?;
    Ok(())
}

/// Compile two codeblocks and diff them
#[poise::command(prefix_command, track_edits, rename = "asmdiff")]
pub async fn run_asmdiff(
    cx: PoiseContext<'_>,
    block1: CodeBlock,
    block2: CodeBlock,
) -> anyhow::Result<()> {
    let State { reqwest, .. } = &**cx.data();
    let response1 = godbolt::get_asm(reqwest, block1.code).await?;
    let response2 = godbolt::get_asm(reqwest, block2.code).await?;

    cx.say(util::codeblock_with_lang(
        "diff",
        &response1.diff(response2),
    ))
    .await?;
    Ok(())
}

const MAX_TIME: Duration = Duration::from_secs(5);

#[poise::command(prefix_command, track_edits, rename = "js")]
pub async fn run_js(cx: PoiseContext<'_>, block: CodeBlock) -> anyhow::Result<()> {
    tracing::info!(%block.code);

    let ClientMessage::EvalResponse(message) = cx
        .data()
        .workers
        .send_timeout(
            HostMessage::Eval(block.code),
            MAX_TIME,
            TimeoutAction::Restart,
        )
        .await?;

    cx.say(util::codeblock_with_lang(
        "js",
        match &message {
            Ok(x) => x,
            Err(x) => x,
        },
    ))
    .await?;

    Ok(())
}

#[poise::command(prefix_command, track_edits, rename = "info")]
pub async fn run_info(cx: PoiseContext<'_>) -> anyhow::Result<()> {
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

    cx.say(output).await?;

    Ok(())
}
