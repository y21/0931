use std::sync::Arc;
use std::time::Duration;

use poise::serenity_prelude::GatewayIntents;
use poise::EditTracker;
use poise::Framework;
use poise::FrameworkOptions;
use poise::PrefixFrameworkOptions;

mod commands;
mod godbolt;
mod playground;
mod state;
mod util;

type State = state::State;
type PoiseContext<'a> = poise::Context<'a, Arc<State>, anyhow::Error>;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let token = std::env::var("DISCORD_TOKEN")?;
    let prefix = std::env::var("DISCORD_PREFIX").unwrap_or_else(|_| ",".into());

    tracing::info!("Using prefix {prefix}");

    let framework = Framework::builder()
        .token(token)
        .intents(GatewayIntents::GUILD_MESSAGES | GatewayIntents::MESSAGE_CONTENT)
        .options(FrameworkOptions {
            allowed_mentions: None,
            prefix_options: PrefixFrameworkOptions {
                ignore_bots: false,
                prefix: Some(prefix),
                edit_tracker: Some(EditTracker::for_timespan(Duration::from_secs(3600))),
                execute_self_messages: false,
                ..Default::default()
            },
            commands: vec![
                commands::rust(),
                commands::help(),
                commands::bench(),
                commands::asm(),
                commands::asmdiff(),
                commands::miri(),
                commands::js(),
                commands::info(),
                commands::docs(),
                commands::fuzzy(),
                commands::find(),
                commands::clippy(),
                commands::expand(),
                commands::godbolt(),
                commands::rustc(),
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
