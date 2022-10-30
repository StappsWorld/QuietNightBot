use serenity::{
    builder::CreateApplicationCommand,
    client::Context,
    model::{
        application::interaction::InteractionResponseType,
        prelude::interaction::application_command::ApplicationCommandInteraction,
    },
};

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
    let has_handler = manager.get(guild_id).is_some();

    if has_handler {
        if let Err(e) = manager.remove(guild_id).await {
            eprintln!("Error removing guild from channel list: {:?}", e);
        }

        crate::util::respond_to_interaction(interaction, &http, false, "Left channel").await;
    } else {
        crate::util::respond_to_interaction(interaction, &http, true, "Not in a channel to leave")
            .await;
    }
}

pub fn register(command: &mut CreateApplicationCommand) -> &mut CreateApplicationCommand {
    command
        .name("leave")
        .description("Has the bot leave the voice channel")
}
