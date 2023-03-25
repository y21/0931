use std::time::Duration;

use anyhow::bail;
use anyhow::Context;
use poise::serenity_prelude::GatewayIntents;
use poise::CodeBlock;
use poise::EditTracker;
use poise::Framework;
use poise::FrameworkOptions;
use poise::PrefixFrameworkOptions;

mod playground;
mod state;
mod util;

type State = state::State;
type PoiseContext<'a> = poise::Context<'a, State, anyhow::Error>;

#[poise::command(prefix_command, track_edits, rename = "run")]
async fn run_rust(cx: PoiseContext<'_>, code: CodeBlock) -> anyhow::Result<()> {
    let CodeBlock { code, .. } = code;

    let response = cx.data().run_code(code).await?;
    cx.say(util::codeblock(&response.output())).await?;

    Ok(())
}

#[poise::command(prefix_command, track_edits, rename = "bench")]
async fn run_bench(cx: PoiseContext<'_>, test1: CodeBlock, test2: CodeBlock) -> anyhow::Result<()> {
    let response = cx.data().bench_code(test1.code, test2.code).await?;
    cx.say(util::codeblock(&response.output())).await?;

    Ok(())
}

#[poise::command(prefix_command, track_edits, rename = "ping")]
async fn run_ping(cx: PoiseContext<'_>) -> anyhow::Result<()> {
    cx.say("ping").await?;
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
            commands: vec![run_rust(), run_ping(), run_bench()],
            ..Default::default()
        })
        .setup(|_ctx, _ready, _framework| Box::pin(async move { Ok(State::new()) }));

    tracing::info!("Running poise");
    framework.run().await?;

    Ok(())
}
