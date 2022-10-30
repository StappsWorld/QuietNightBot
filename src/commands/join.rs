use serenity::{
    builder::CreateApplicationCommand,
    client::Context,
    model::{
        application::interaction::InteractionResponseType,
        prelude::interaction::application_command::ApplicationCommandInteraction,
    },
    prelude::Mentionable,
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
    let guild = match ctx.cache.guild(guild_id) {
        Some(guild) => guild,
        None => {
            crate::util::respond_to_interaction(
                interaction,
                &http,
                true,
                "Internal Error: Failed to get guild from cache",
            )
            .await;
            return;
        }
    };

    let channel_id = guild
        .voice_states
        .get(&interaction.user.id)
        .and_then(|voice_state| voice_state.channel_id);

    let connect_to = match channel_id {
        Some(channel) => channel,
        None => {
            crate::util::respond_to_interaction(
                interaction,
                &http,
                true,
                "You must be in a voice channel to use this command",
            )
            .await;
            return;
        }
    };

    let manager = songbird::get(ctx)
        .await
        .expect("Songbird Voice client placed in at initialisation.")
        .clone();

    let (_, success) = manager.join(guild_id, connect_to).await;

    match success {
        Ok(_) => {
            crate::util::respond_to_interaction(
                interaction,
                &http,
                false,
                format!("Joined {}", connect_to.mention()),
            )
            .await;
        }
        Err(e) => {
            eprintln!("Failed to join voice channel: {}", e);
            crate::util::respond_to_interaction(
                interaction,
                &http,
                true,
                "Failed to join voice channel",
            )
            .await;
        }
    }
}

pub fn register(command: &mut CreateApplicationCommand) -> &mut CreateApplicationCommand {
    command
        .name("join")
        .description("Has the bot join your voice channel")
}
