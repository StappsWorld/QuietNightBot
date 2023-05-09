extern crate lazy_static;

use dotenv::dotenv;
use serenity::model::application::interaction::Interaction;
use serenity::model::prelude::*;
use serenity::prelude::*;
use serenity::{async_trait, client::Client};
use songbird::SerenityInit;
use std::env;

pub mod commands;
pub mod events;
pub mod util;

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        match interaction {
            Interaction::ApplicationCommand(command) => match command.data.name.as_str() {
                "join" => commands::join::run(&ctx, &command).await,
                "leave" => commands::leave::run(&ctx, &command).await,
                "mute" => commands::mute::run(&ctx, &command).await,
                "ping" => commands::ping::run(&ctx, &command).await,
                "queue" => commands::queue::run(&ctx, &command).await,
                "skip" => commands::skip::run(&ctx, &command).await,
                "stop" => commands::stop::run(&ctx, &command).await,
                "unmute" => commands::unmute::run(&ctx, &command).await,
                "search" => commands::search::run(&ctx, &command).await,
                "setrain" => commands::setrain::run(&ctx, &command).await,
                "setvolume" => commands::setvolume::run(&ctx, &command).await,
                _ => {
                    match crate::util::respond_to_interaction(
                        &command,
                        &ctx.http,
                        true,
                        "Unknown command",
                    )
                    .await
                    {
                        Some(_) => (),
                        None => eprintln!("Failed to respond to unknown interaction"),
                    };
                }
            },
            _ => (),
        }
    }

    async fn voice_state_update(&self, ctx: Context, old: Option<VoiceState>, new: VoiceState) {
        crate::events::voice_state_update::voice_state_update(ctx, old, new).await;
    }

    async fn ready(&self, ctx: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name);

        let guilds = ctx.cache.guilds();
        let mut rain_enabled_map = match crate::util::RAIN_ENABLED.try_lock() {
            Ok(map) => map,
            Err(e) => {
                eprintln!("Failed to lock RAIN_ENABLED map with error {}", e);
                return;
            }
        };

        for guild_id in guilds {
            match guild_id
                .set_application_commands(&ctx.http, |commands| {
                    commands
                        .create_application_command(|command| commands::join::register(command))
                        .create_application_command(|command| commands::leave::register(command))
                        .create_application_command(|command| commands::mute::register(command))
                        .create_application_command(|command| commands::ping::register(command))
                        .create_application_command(|command| commands::queue::register(command))
                        .create_application_command(|command| commands::skip::register(command))
                        .create_application_command(|command| commands::stop::register(command))
                        .create_application_command(|command| commands::unmute::register(command))
                        .create_application_command(|command| commands::search::register(command))
                        .create_application_command(|command| commands::setrain::register(command))
                })
                .await
            {
                Ok(_) => println!("Registered slash commands for guild {}", guild_id),
                Err(why) => eprintln!(
                    "Failed to register slash commands for guild {}: {}",
                    guild_id, why
                ),
            }
            rain_enabled_map.insert(guild_id.to_string(), true);
        }
    }
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    match dotenv() {
        Ok(_) => (),
        Err(e) => eprintln!("Failed to open .env: {}", e),
    }

    // Configure the client with your Discord bot token in the environment.
    let token = env::var("DISCORD_TOKEN").expect("Expected a token in the environment");

    let intents = GatewayIntents::non_privileged() | GatewayIntents::MESSAGE_CONTENT;

    let mut client = Client::builder(&token, intents)
        .event_handler(Handler)
        .register_songbird()
        .await
        .expect("Err creating client");

    let _ = client
        .start()
        .await
        .map_err(|why| println!("Client ended: {:?}", why));
}
