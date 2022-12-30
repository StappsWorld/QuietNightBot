use serenity::builder::CreateApplicationCommand;
use serenity::model::application::interaction::InteractionResponseType;
use serenity::model::prelude::interaction::application_command::ApplicationCommandInteraction;
use serenity::prelude::*;
use tokio::time::timeout;

pub async fn run(ctx: &Context, interaction: &ApplicationCommandInteraction) {
    let guild_id = match interaction.guild_id {
        Some(id) => id,
        None => {
            match interaction
                .create_interaction_response(&ctx.http, |r| {
                    r.kind(InteractionResponseType::ChannelMessageWithSource)
                        .interaction_response_data(|m| {
                            m.content("This command can only be used in a server")
                        })
                })
                .await
            {
                Ok(_) => return,
                Err(e) => {
                    eprintln!("Cannot respond to slash command: {}", e);
                    return;
                }
            }
        }
    };
    let http = ctx.http.clone();

    let manager = songbird::get(ctx)
        .await
        .expect("Songbird Voice client placed in at initialisation.")
        .clone();

    if let Some(handler_lock) = manager.get(guild_id) {
        let handler = match timeout(std::time::Duration::from_secs(5), handler_lock.lock()).await {
            Ok(handler) => handler,
            Err(e) => {
                eprintln!("Failed to lock handler with error {}", e);
                crate::util::respond_to_interaction(
                    interaction,
                    &ctx.http,
                    true,
                    "There was an error. Please try again later.",
                )
                .await;

                return;
            }
        };
        let queue = handler.queue();
        let _ = queue.stop();
        crate::util::respond_to_interaction(interaction, &http, false, "Queue cleared").await;
    } else {
        crate::util::respond_to_interaction(
            interaction,
            &http,
            true,
            "Not in a voice channel to clear in",
        )
        .await;
    }
}

pub fn register(command: &mut CreateApplicationCommand) -> &mut CreateApplicationCommand {
    command
        .name("stop")
        .description("Stops the bot from playing music. Clears the queue.")
}
