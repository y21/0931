use std::fmt::Write;
use std::sync::Arc;
use std::time::Duration;

use human_size::Byte;
use human_size::Megabyte;
use human_size::SpecificSize;
use ipc2_host::workerset::TimeoutAction;
use poise::samples::HelpConfiguration;
use poise::serenity_prelude::GatewayIntents;
use poise::CodeBlock;
use poise::EditTracker;
use poise::Framework;
use poise::FrameworkOptions;
use poise::PrefixFrameworkOptions;
use shared::ClientMessage;
use shared::HostMessage;
use sysinfo::CpuExt;
use sysinfo::SystemExt;

mod godbolt;
mod playground;
mod state;
mod util;

type State = state::State;
type PoiseContext<'a> = poise::Context<'a, Arc<State>, anyhow::Error>;

/// Executes a Rust codeblock
///
/// The code can simply be an expression and the bot will automatically
/// wrap it in a main function and a print statement.
#[poise::command(prefix_command, track_edits, broadcast_typing, rename = "run")]
async fn run_rust(cx: PoiseContext<'_>, code: CodeBlock) -> anyhow::Result<()> {
    let CodeBlock { code, .. } = code;

    let response = cx.data().run_code(code).await?;
    cx.say(util::codeblock(util::strip_header_stderr(
        &response.output(),
    )))
    .await?;

    Ok(())
}

/// Benchmarks two Rust codeblocks to see which one runs faster
#[poise::command(prefix_command, track_edits, broadcast_typing, rename = "bench")]
async fn run_bench(cx: PoiseContext<'_>, test1: CodeBlock, test2: CodeBlock) -> anyhow::Result<()> {
    let response = cx.data().bench_code(test1.code, test2.code).await?;
    cx.say(util::codeblock(util::strip_header_stderr(
        &response.output(),
    )))
    .await?;

    Ok(())
}

/// Runs a codeblock under miri, an interpreter that checks for memory errors
#[poise::command(prefix_command, track_edits, broadcast_typing, rename = "miri")]
async fn run_miri(cx: PoiseContext<'_>, block: CodeBlock) -> anyhow::Result<()> {
    let response = playground::run_miri(&cx.data().reqwest, block.code).await?;
    cx.say(util::codeblock(util::strip_header_stderr(
        &response.output(),
    )))
    .await?;

    Ok(())
}

/// Help me
#[poise::command(prefix_command, track_edits, rename = "help")]
async fn run_help(cx: PoiseContext<'_>, command: Option<String>) -> anyhow::Result<()> {
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
async fn run_asm(cx: PoiseContext<'_>, blocks: Vec<CodeBlock>) -> anyhow::Result<()> {
    let mut output = String::new();

    for block in blocks {
        let out = cx.data().get_asm(block.code).await?;
        output.push_str(&util::codeblock(&out.0));
    }

    cx.say(&output).await?;
    Ok(())
}

/// Compile two codeblocks and diff them
#[poise::command(prefix_command, track_edits, rename = "asmdiff")]
async fn run_asmdiff(cx: PoiseContext<'_>, cb1: CodeBlock, cb2: CodeBlock) -> anyhow::Result<()> {
    let data = cx.data();
    let response1 = data.get_asm(cb1.code).await?;
    let response2 = data.get_asm(cb2.code).await?;

    cx.say(util::codeblock_with_lang(
        "diff",
        &response1.diff(response2),
    ))
    .await?;
    Ok(())
}

const MAX_TIME: Duration = Duration::from_secs(5);

#[poise::command(prefix_command, track_edits, rename = "js")]
async fn run_js(cx: PoiseContext<'_>, cb: CodeBlock) -> anyhow::Result<()> {
    tracing::info!(%cb.code);

    let ClientMessage::EvalResponse(message) = cx
        .data()
        .workers
        .send_timeout(HostMessage::Eval(cb.code), MAX_TIME, TimeoutAction::Restart)
        .await?;

    let message = util::codeblock_with_lang(
        "js",
        match &message {
            Ok(x) => x,
            Err(x) => x,
        },
    );
    cx.say(message).await?;

    Ok(())
}

#[poise::command(prefix_command, track_edits, rename = "info")]
async fn run_info(cx: PoiseContext<'_>) -> anyhow::Result<()> {
    let output = {
        let temp = util::get_temp()?
            .map(|t| t.to_string())
            .unwrap_or_else(|| "<unsupported>".into());

        let mut sys = cx.data().system.lock().unwrap();
        sys.refresh_all();

        let mut output = format!("```\nTemperature: {temp}\n");

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

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let token = std::env::var("DISCORD_TOKEN")?;

    let framework = Framework::builder()
        .token(token)
        .intents(GatewayIntents::GUILD_MESSAGES | GatewayIntents::MESSAGE_CONTENT)
        .options(FrameworkOptions {
            allowed_mentions: None,
            prefix_options: PrefixFrameworkOptions {
                prefix: Some(",".into()),
                edit_tracker: Some(EditTracker::for_timespan(Duration::from_secs(3600))),
                ..Default::default()
            },
            commands: vec![
                run_rust(),
                run_help(),
                run_bench(),
                run_asm(),
                run_asmdiff(),
                run_miri(),
                run_js(),
                run_info(),
            ],
            ..Default::default()
        })
        .setup(|_ctx, _ready, _framework| {
            Box::pin(async move { Ok(Arc::new(State::new().await?)) })
        });

    tracing::info!("Running poise");
    framework.run().await?;

    Ok(())
}
