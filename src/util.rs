use lazy_static::lazy_static;
use serenity::{
    http::client::Http,
    model::prelude::interaction::application_command::ApplicationCommandInteraction,
};
use std::sync::Arc;

lazy_static! {
    pub static ref YOUTUBE_URL_REGEX: regex::Regex = regex::Regex::new(
        r"^(?:https?://)?(?:www\.)?(?:youtu\.be/|youtube\.com/(?:embed/|v/|watch\?v=|watch\?.+&v=))(?P<video_id>[\w-]{11})(?:\S+)?$"
    ).unwrap();
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
