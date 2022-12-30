use lazy_static::lazy_static;
use serenity::model::application::interaction::InteractionResponseType;
use serenity::prelude::*;
use serenity::{
    http::client::Http,
    model::prelude::interaction::application_command::ApplicationCommandInteraction,
};
use songbird::input::Restartable;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::time::timeout;
use yt_api::ApiKey;

lazy_static! {
    pub static ref YOUTUBE_URL_REGEX: regex::Regex = regex::Regex::new(
        r"^(?:https?://)?(?:www\.)?(?:youtu\.be/|youtube\.com/(?:embed/|v/|watch\?v=|watch\?.+&v=))(?P<video_id>[\w-]{11})(?:\S+)?$"
    ).expect("Failed to compile YouTube URL regex");
    pub static ref YOUTUBE_API_KEY: ApiKey = ApiKey::new(std::env::var("YOUTUBE_API_KEY").expect("YOUTUBE_API_KEY not set"));
    pub static ref RAIN_ENABLED: Arc<Mutex<HashMap<String, bool>>> = Arc::new(Mutex::new(HashMap::new()));
}

pub async fn respond_to_interaction<S: ToString>(
    interaction: &ApplicationCommandInteraction,
    http: &Arc<Http>,
    ephemeral: bool,
    content: S,
) -> Option<()> {
    match interaction
        .create_interaction_response(http, |create| {
            create.interaction_response_data(|data| {
                data.content(content.to_string()).ephemeral(ephemeral)
            })
        })
        .await
    {
        Ok(_) => return Some(()),
        Err(e) => {
            eprintln!("Cannot respond to slash command: {}", e);
            return None;
        }
    };
}

pub async fn follow_up_interaction<S: ToString>(
    interaction: &ApplicationCommandInteraction,
    http: &Arc<Http>,
    ephemeral: bool,
    content: S,
) -> Option<()> {
    match interaction
        .create_followup_message(http, |create| {
            create.content(content.to_string()).ephemeral(ephemeral)
        })
        .await
    {
        Ok(_) => return Some(()),
        Err(e) => {
            eprintln!("Cannot respond to slash command: {}", e);
            return None;
        }
    };
}

pub async fn play_song(ctx: &Context, interaction: &ApplicationCommandInteraction, url: &str) {
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

    let manager = songbird::get(ctx)
        .await
        .expect("Songbird Voice client placed in at initialisation.")
        .clone();

    let handler_lock = match manager.get(guild_id) {
        Some(handler_lock) => handler_lock,
        None => {
            let guild = match ctx.cache.guild(guild_id) {
                Some(guild) => guild,
                None => {
                    crate::util::respond_to_interaction(
                        interaction,
                        &ctx.http,
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
                        &ctx.http,
                        true,
                        "You must be in a voice channel to use this command",
                    )
                    .await;
                    return;
                }
            };
            let (handler_lock, success) = manager.join(guild_id, connect_to).await;
            match success {
                Ok(_) => handler_lock,
                Err(e) => {
                    eprintln!("Failed to join voice channel: {}", e);
                    crate::util::respond_to_interaction(
                        interaction,
                        &ctx.http,
                        true,
                        "Failed to join voice channel",
                    )
                    .await;
                    return;
                }
            }
        }
    };

    crate::util::respond_to_interaction(interaction, &ctx.http, true, "Beginning to queue song")
        .await;

    let rain_enabled = match crate::util::RAIN_ENABLED.try_lock() {
        Ok(map) => *map.get(&guild_id.to_string()).unwrap_or(&false),
        Err(e) => {
            eprintln!("Failed to lock RAIN_ENABLED map with error {}", e);
            false
        }
    };
    let norain_source_path_str = format!("./queue/norain_{}.mp3", video_id);
    let norain_source_path = std::path::Path::new(&norain_source_path_str);

    if !norain_source_path.exists() {
        // Make queue folder if it doesn't exist
        let queue_folder = std::path::Path::new("queue");
        if !queue_folder.exists() {
            std::fs::create_dir(queue_folder).expect("Failed to create queue folder");
        }

        let download_command = format!(
            "yt-dlp -f 'ba' -x --audio-format mp3 \'{}\' -o \'{}\'",
            url, norain_source_path_str
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
                    crate::util::follow_up_interaction(
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

                crate::util::follow_up_interaction(
                    interaction,
                    &ctx.http,
                    true,
                    "Error downloading video/audio",
                )
                .await;

                return;
            }
        }
    }

    let audio_source = if rain_enabled {
        let rain_source_path_str = format!("./queue/{}.mp3", video_id);
        let rain_source_path = std::path::Path::new(&rain_source_path_str);

        if !rain_source_path.exists() {
            crate::util::follow_up_interaction(interaction, &ctx.http, true, "Encoding song").await;

            // Download/mix the video/audio into a single source.

            let rain_path = match std::env::var("RAIN_PATH") {
                Ok(path) => path,
                Err(e) => {
                    eprintln!("Failed to get RAIN_PATH: {}", e);
                    crate::util::follow_up_interaction(
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
                    rain_path, norain_source_path_str, rain_source_path_str
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
                        crate::util::follow_up_interaction(
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

                    crate::util::follow_up_interaction(
                        interaction,
                        &ctx.http,
                        true,
                        "Error mixing audio",
                    )
                    .await;

                    return;
                }
            }
        }
        rain_source_path_str
    } else {
        norain_source_path_str
    };

    // Here, we use lazy restartable sources to make sure that we don't pay
    // for decoding, playback on tracks which aren't actually live yet.
    let source = match Restartable::ffmpeg(audio_source, true).await {
        Ok(source) => source,
        Err(why) => {
            eprintln!("Err starting source: {:?}", why);
            crate::util::follow_up_interaction(
                interaction,
                &ctx.http,
                true,
                "Error sourcing ffmpeg",
            )
            .await;

            return;
        }
    };

    let queue_len = {
        let mut handler =
            match timeout(std::time::Duration::from_secs(5), handler_lock.lock()).await {
                Ok(handler) => handler,
                Err(e) => {
                    eprintln!("Failed to lock handler with error {}", e);
                    crate::util::follow_up_interaction(
                        interaction,
                        &ctx.http,
                        true,
                        "There was an error adding the song to the queue. Please try again later.",
                    )
                    .await;

                    return;
                }
            };
        handler.enqueue_source(source.into());
        handler.queue().len()
    };

    crate::util::follow_up_interaction(
        interaction,
        &ctx.http,
        false,
        format!(
            "User {} added song {} to queue: position {} (rain enabled: {})",
            interaction.user.tag(),
            url,
            queue_len,
            rain_enabled
        ),
    )
    .await;
}
