use serenity::{
    builder::CreateApplicationCommand,
    client::Context,
    model::{
        application::interaction::InteractionResponseType,
        prelude::interaction::application_command::ApplicationCommandInteraction,
    },
};
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

    let handler_lock = match manager.get(guild_id) {
        Some(handler) => handler,
        None => {
            crate::util::respond_to_interaction(interaction, &http, true, "Not in a voice channel")
                .await;
            return;
        }
    };

    let mut handler = match timeout(std::time::Duration::from_secs(5), handler_lock.lock()).await {
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

    if handler.is_mute() {
        crate::util::respond_to_interaction(interaction, &http, true, "Already muted").await;
    } else {
        if let Err(e) = handler.mute(true).await {
            eprintln!("Failed to mute: {:?}", e);
            crate::util::respond_to_interaction(interaction, &http, true, "Failed to mute").await;
        } else {
            crate::util::respond_to_interaction(interaction, &http, false, "Now muted").await;
        }
    }
}

pub fn register(command: &mut CreateApplicationCommand) -> &mut CreateApplicationCommand {
    command
        .name("mute")
        .description("Has the bot mute itself. The song will continue playing if there is a song currently playing.")
}
