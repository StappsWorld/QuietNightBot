use serenity::builder::CreateApplicationCommand;
use serenity::model::application::interaction::InteractionResponseType;
use serenity::model::prelude::command::CommandOptionType;
use serenity::model::prelude::interaction::application_command::{
    ApplicationCommandInteraction, CommandDataOptionValue,
};
use serenity::prelude::*;
use yt_api::search::*;

pub async fn run(ctx: &Context, interaction: &ApplicationCommandInteraction) {
    match interaction.guild_id {
        Some(_) => (),
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

    let query = match interaction.data.options.get(0) {
        Some(query_option) => {
            if query_option.name != "query".to_owned() {
                crate::util::respond_to_interaction(
                    interaction,
                    &ctx.http,
                    true,
                    "Internal Error: Failed to get query from interaction",
                )
                .await;
                return;
            } else {
                match &query_option.resolved {
                    Some(resolved) => match resolved {
                        CommandDataOptionValue::String(query) => query,
                        x => {
                            eprintln!("Unexpected type for query: {:?}", x);
                            crate::util::respond_to_interaction(
                                interaction,
                                &ctx.http,
                                true,
                                "Internal Error: Failed to get query from interaction",
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
                            "Internal Error: Failed to get query from interaction",
                        )
                        .await;
                        return;
                    }
                }
            }
        }
        None => {
            eprintln!(
                "interaction.data.options length was 0. Could not get query from interaction"
            );

            crate::util::respond_to_interaction(
                interaction,
                &ctx.http,
                true,
                "Must provide a query",
            )
            .await;

            return;
        }
    };

    let result = match SearchList::new(crate::util::YOUTUBE_API_KEY.clone())
        .q(query)
        .item_type(ItemType::Video)
        .await
    {
        Ok(result) => result,
        Err(e) => {
            eprintln!("Failed to search YouTube: {}", e);
            crate::util::respond_to_interaction(
                interaction,
                &ctx.http,
                true,
                "Failed to search YouTube",
            )
            .await;
            return;
        }
    };

    let video_id = match result.items.get(0) {
        Some(video) => match &video.id.video_id {
            Some(id) => id,
            None => {
                eprintln!("No video ID found in search result");
                crate::util::respond_to_interaction(interaction, &ctx.http, true, "No video found")
                    .await;
                return;
            }
        },
        None => {
            crate::util::respond_to_interaction(interaction, &ctx.http, true, "No videos found")
                .await;
            return;
        }
    };

    let url = format!("https://www.youtube.com/watch?v={}", video_id);

    crate::util::play_song(ctx, interaction, url.as_str()).await;
}

pub fn register(command: &mut CreateApplicationCommand) -> &mut CreateApplicationCommand {
    command
        .name("search")
        .description("Search for songs to play")
        .create_option(|option| {
            option
                .name("query")
                .description("Name of the video to queue")
                .kind(CommandOptionType::String)
                .required(true)
        })
}
