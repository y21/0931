use std::time::Duration;

use poise::samples::HelpConfiguration;
use poise::serenity_prelude::GatewayIntents;
use poise::CodeBlock;
use poise::EditTracker;
use poise::Framework;
use poise::FrameworkOptions;
use poise::PrefixFrameworkOptions;

mod godbolt;
mod playground;
mod state;
mod util;

type State = state::State;
type PoiseContext<'a> = poise::Context<'a, State, anyhow::Error>;

/// Executes a Rust codeblock
///
/// The code can simply be an expression and the bot will automatically
/// wrap it in a main function and a print statement.
#[poise::command(prefix_command, track_edits, broadcast_typing, rename = "run")]
async fn run_rust(cx: PoiseContext<'_>, code: CodeBlock) -> anyhow::Result<()> {
    let CodeBlock { code, .. } = code;

    let response = cx.data().run_code(code).await?;
    cx.say(util::codeblock(&response.output())).await?;

    Ok(())
}

/// Benchmarks two Rust codeblocks to see which one runs faster
#[poise::command(prefix_command, track_edits, broadcast_typing, rename = "bench")]
async fn run_bench(cx: PoiseContext<'_>, test1: CodeBlock, test2: CodeBlock) -> anyhow::Result<()> {
    let response = cx.data().bench_code(test1.code, test2.code).await?;
    cx.say(util::codeblock(&response.output())).await?;

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
            ],
            ..Default::default()
        })
        .setup(|_ctx, _ready, _framework| Box::pin(async move { Ok(State::new()) }));

    tracing::info!("Running poise");
    framework.run().await?;

    Ok(())
}
