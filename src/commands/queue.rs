use serenity::builder::CreateApplicationCommand;
use serenity::model::application::{
    command::CommandOptionType, interaction::application_command::CommandDataOptionValue,
};
use serenity::model::prelude::interaction::application_command::ApplicationCommandInteraction;
use serenity::prelude::*;

pub async fn run(ctx: &Context, interaction: &ApplicationCommandInteraction) {
    let url = match interaction.data.options.get(0) {
        Some(url_option) => {
            if url_option.name != "url".to_owned() {
                crate::util::respond_to_interaction(
                    interaction,
                    &ctx.http,
                    true,
                    "Internal Error: Failed to get url from interaction",
                )
                .await;
                return;
            } else {
                match &url_option.resolved {
                    Some(resolved) => match resolved {
                        CommandDataOptionValue::String(url) => url,
                        x => {
                            eprintln!("Unexpected type for url: {:?}", x);
                            crate::util::respond_to_interaction(
                                interaction,
                                &ctx.http,
                                true,
                                "Internal Error: Failed to get url from interaction",
                            )
                            .await;
                            return;
                        }
                    },
                    None => {
                        eprintln!("No url provided");
                        crate::util::respond_to_interaction(
                            interaction,
                            &ctx.http,
                            true,
                            "Internal Error: Failed to get url from interaction",
                        )
                        .await;
                        return;
                    }
                }
            }
        }
        None => {
            eprintln!("interaction.data.options length was 0. Could not get URL from interaction");

            crate::util::respond_to_interaction(
                interaction,
                &ctx.http,
                true,
                "Must provide a valid YouTube URL",
            )
            .await;

            return;
        }
    };

    crate::util::play_song(ctx, interaction, url).await;
}

pub fn register(command: &mut CreateApplicationCommand) -> &mut CreateApplicationCommand {
    command
        .name("queue")
        .description("Queues a song to play in the voice channel")
        .create_option(|option| {
            option
                .name("url")
                .description("URL of the song to queue")
                .kind(CommandOptionType::String)
                .required(true)
        })
}
