use serenity::builder::CreateApplicationCommand;
use serenity::model::application::interaction::InteractionResponseType;
use serenity::model::prelude::interaction::application_command::ApplicationCommandInteraction;
use serenity::prelude::*;

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
        let handler = handler_lock.lock().await;
        let queue = handler.queue();
        let _ = queue.skip();
        crate::util::respond_to_interaction(
            interaction,
            &http,
            false,
            format!("Song skipped: {} in queue.", queue.len()),
        )
        .await;
    } else {
        crate::util::respond_to_interaction(
            interaction,
            &http,
            true,
            "Not in a voice channel to play in",
        )
        .await;
    }
}

pub fn register(command: &mut CreateApplicationCommand) -> &mut CreateApplicationCommand {
    command.name("skip").description("Skips the current song")
}
