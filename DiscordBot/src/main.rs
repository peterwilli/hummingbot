mod args;
mod config;
mod trade;
use anyhow::anyhow;
use anyhow::Result;
use args::Args;
use clap::Parser;
use config::BotEntry;
use config::Config;
use debounced::debounced;
use futures::SinkExt;
use futures::StreamExt;
use log::debug;
use log::warn;
use notify::{Config as NotifyConfig, RecommendedWatcher, RecursiveMode, Watcher};
use poise::serenity_prelude as serenity;
use poise::serenity_prelude::ChannelId;
use poise::serenity_prelude::CreateEmbed;
use poise::serenity_prelude::CreateMessage;
use std::borrow::Cow;
use std::fs;
use std::io::Read;
use std::io::Seek;
use std::io::SeekFrom;
use std::path::PathBuf;
use std::time::Duration;
use tokio::sync::mpsc;

use crate::trade::Trade;
use crate::trade::TradeSide;

struct Data {} // User data, which is stored and accessible in all command invocations
type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, Data, Error>;

/// Displays your or another user's account creation date
#[poise::command(slash_command, prefix_command)]
async fn age(
    ctx: Context<'_>,
    #[description = "Selected user"] user: Option<serenity::User>,
) -> Result<(), Error> {
    let u = user.as_ref().unwrap_or_else(|| ctx.author());
    let response = format!("{}'s account was created at {}", u.name, u.created_at());
    ctx.say(response).await?;
    Ok(())
}

async fn notify_trade<'c>(
    ctx: &poise::serenity_prelude::Context,
    bot_name: &str,
    channel: &ChannelId,
    trade: Trade<'c>,
) -> Result<()> {
    let embed = CreateEmbed::new()
        .title("New trade")
        .color(match trade.side {
            TradeSide::Buy => 0x41d321,
            TradeSide::Sell => 0xd32121,
        })
        .description(format!(
            "{} {}/{}",
            trade.side, trade.base_asset, trade.quote_asset
        ))
        .fields(vec![
            ("Bot", bot_name, false),
            ("Amount", &trade.amount, true),
            (
                "Price",
                format!("{} {}", trade.price, trade.quote_asset).as_ref(),
                true,
            ),
        ]);
    let builder = CreateMessage::new().add_embed(embed);
    channel.send_message(ctx, builder).await?;
    return Ok(());
}

async fn trade_loop<'c>(ctx: &poise::serenity_prelude::Context, config: &Config<'c>) -> Result<()> {
    for bot in config.bots.iter() {
        let (mut tx, rx) = futures::channel::mpsc::channel(16);
        let mut debounced = debounced(rx, Duration::from_secs(1));
        // Automatically select the best implementation for your platform.
        // You can also access each implementation directly e.g. INotifyWatcher.
        let mut watcher = RecommendedWatcher::new(
            move |res| {
                futures::executor::block_on(async {
                    tx.send(res).await.unwrap();
                })
            },
            NotifyConfig::default(),
        )?;

        let mut last_file_len = fs::metadata(&bot.trades_path).unwrap().len();
        let trades_path = bot.trades_path.clone();
        let ctx = ctx.clone();
        let stats_channel = ChannelId::new(config.stats_channel_id);
        let bot_name = bot.name.to_string();
        tokio::spawn(async move {
            watcher
                .watch(&trades_path, RecursiveMode::NonRecursive)
                .unwrap();
            loop {
                let event = debounced.next().await;
                if event.is_none() {
                    continue;
                }
                let mut file = fs::File::open(&trades_path).unwrap();
                file.seek(SeekFrom::Start(last_file_len)).unwrap();
                let mut buf = vec![0u8; (file.metadata().unwrap().len() - last_file_len) as usize];
                file.read_exact(&mut buf).unwrap();
                last_file_len = file.metadata().unwrap().len();
                let line = String::from_utf8_lossy(&buf);
                drop(file);
                let components = line.split(",").collect::<Vec<&str>>();
                let trade = Trade {
                    base_asset: components[5].into(),
                    quote_asset: components[6].into(),
                    amount: components[12].into(),
                    price: components[11].into(),
                    side: if components[9] == "BUY" {
                        TradeSide::Buy
                    } else {
                        TradeSide::Sell
                    },
                };
                notify_trade(&ctx, &bot_name, &stats_channel, trade)
                    .await
                    .unwrap();
            }
        });
        debug!("Set up watcher for {}", bot.trades_path.display());
    }

    loop {
        tokio::time::sleep(Duration::from_secs(10)).await;
    }
}

fn init_config<'c>(path: &PathBuf) -> Result<Config<'c>> {
    if path.exists() {
        let bytes = std::fs::read(path)?;
        let contents = String::from_utf8_lossy(&bytes);
        let config: Config = serde_yaml::from_str(&contents)?;
        return Ok(config);
    } else {
        let default_config = Config::default();
        let yaml_config = serde_yaml::to_string(&default_config)?;
        std::fs::write(&path, &yaml_config)?;
        return Err(anyhow!(
            "No config file found, default file created at {}!",
            path.display()
        ));
    }
}

#[tokio::main]
async fn main() {
    env_logger::init();
    let args = Args::parse();
    let config = init_config(&args.config_path).unwrap();
    let intents = serenity::GatewayIntents::non_privileged();
    let bot_token = config.bot_token.clone();

    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: vec![age()],
            ..Default::default()
        })
        .setup(move |ctx, _ready, framework| {
            Box::pin(async move {
                poise::builtins::register_globally(ctx, &framework.options().commands).await?;
                trade_loop(ctx, &config).await?;
                Ok(Data {})
            })
        })
        .build();

    let client = serenity::ClientBuilder::new(&bot_token, intents)
        .framework(framework)
        .await;
    client.unwrap().start().await.unwrap();
}
