use serenity::builder::CreateApplicationCommand;
use serenity::model::application::{
    command::CommandOptionType, interaction::application_command::CommandDataOptionValue,
    interaction::InteractionResponseType,
};
use serenity::model::prelude::interaction::application_command::ApplicationCommandInteraction;
use serenity::prelude::*;
use songbird::input::restartable::Restartable;

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

    // regex for youtube video id
    let re = &crate::util::YOUTUBE_URL_REGEX;

    if !re.is_match(&url) {
        crate::util::respond_to_interaction(
            interaction,
            &ctx.http,
            true,
            "Must provide a valid YouTube URL",
        )
        .await;

        return;
    }

    let video_id = re
        .captures(&url)
        .unwrap()
        .name("video_id")
        .unwrap()
        .as_str();

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

    let manager = songbird::get(ctx)
        .await
        .expect("Songbird Voice client placed in at initialisation.")
        .clone();

    if let Some(handler_lock) = manager.get(guild_id) {
        let mut handler = handler_lock.lock().await;

        let source_path_str = format!("./queue/{}.mp3", video_id);
        let source_path = std::path::Path::new(&source_path_str);

        if !source_path.exists() {
            // Make queue folder if it doesn't exist
            let queue_folder = std::path::Path::new("queue");
            if !queue_folder.exists() {
                std::fs::create_dir(queue_folder).expect("Failed to create queue folder");
            }

            // Download/mix the video/audio into a single source.
            let source_unmixed_path = format!("./queue/unmixed_{}.mp3", video_id);
            let download_command = format!(
                "yt-dlp -f 'ba' -x --audio-format mp3 \'{}\' -o \'{}\'",
                url, source_unmixed_path
            );
            match std::process::Command::new("sh")
                .arg("-c")
                .arg(&download_command)
                .output()
            {
                Ok(output) => {
                    if !output.status.success() {
                        eprintln!(
                            "Failed to download video: {}",
                            String::from_utf8_lossy(&output.stderr)
                        );
                        crate::util::respond_to_interaction(
                            interaction,
                            &ctx.http,
                            true,
                            "Error downloading video/audio",
                        )
                        .await;

                        return;
                    }
                }
                Err(e) => {
                    eprintln!("failed to execute yt-dlp process: {:?}", e);
                    eprintln!("Command: {}", download_command);

                    crate::util::respond_to_interaction(
                        interaction,
                        &ctx.http,
                        true,
                        "Error downloading video/audio",
                    )
                    .await;

                    return;
                }
            }

            let rain_path = match std::env::var("RAIN_PATH") {
                Ok(path) => path,
                Err(e) => {
                    eprintln!("Failed to get RAIN_PATH: {}", e);
                    crate::util::respond_to_interaction(
                        interaction,
                        &ctx.http,
                        true,
                        "Internal Error... Please try again later",
                    )
                    .await;

                    return;
                }
            };
            let mix_command = format!(
                "ffmpeg -stream_loop -1 -i \"{}\" -i \"{}\"  -filter_complex \"[0:a]volume=0.75[a0];[1:a]volume=1[a1];[a0][a1]amerge[a]\" -map \"[a]\" -ac 2 \"{}\"",
                rain_path, source_unmixed_path, source_path_str
            );
            match std::process::Command::new("sh")
                .arg("-c")
                .arg(&mix_command)
                .output()
            {
                Ok(output) => {
                    if !output.status.success() {
                        eprintln!(
                            "Failed to mix audio: {}",
                            String::from_utf8_lossy(&output.stderr)
                        );
                        crate::util::respond_to_interaction(
                            interaction,
                            &ctx.http,
                            true,
                            "Error mixing audio",
                        )
                        .await;

                        return;
                    }
                }
                Err(e) => {
                    eprintln!("failed to spawn ffmpeg to mix audio with rain: {}", e);
                    eprintln!("Command: {}", mix_command);

                    crate::util::respond_to_interaction(
                        interaction,
                        &ctx.http,
                        true,
                        "Error mixing audio",
                    )
                    .await;

                    return;
                }
            }

            match std::fs::remove_file(source_unmixed_path) {
                Ok(_) => {}
                Err(e) => {
                    eprintln!("Failed to remove unmixed source file: {}", e);
                }
            }
        }

        // Here, we use lazy restartable sources to make sure that we don't pay
        // for decoding, playback on tracks which aren't actually live yet.
        let source = match Restartable::ffmpeg(source_path_str, true).await {
            Ok(source) => source,
            Err(why) => {
                eprintln!("Err starting source: {:?}", why);
                crate::util::respond_to_interaction(
                    interaction,
                    &ctx.http,
                    true,
                    "Error sourcing ffmpeg",
                )
                .await;

                return;
            }
        };

        handler.enqueue_source(source.into());

        crate::util::respond_to_interaction(
            interaction,
            &ctx.http,
            false,
            format!("Added song to queue: position {}", handler.queue().len()),
        )
        .await;
    } else {
        crate::util::respond_to_interaction(
            interaction,
            &ctx.http,
            true,
            "Not in a voice channel to play in",
        )
        .await;
    }
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
