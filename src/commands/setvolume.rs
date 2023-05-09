use serenity::{
    builder::CreateApplicationCommand,
    client::Context,
    model::{
        application::command::CommandOptionType,
        prelude::interaction::application_command::ApplicationCommandInteraction,
    },
};
use tokio::time::timeout;

pub async fn run(ctx: &Context, interaction: &ApplicationCommandInteraction) {
    let http = ctx.http.clone();
    let volume = match interaction.data.options.get(0) {
        Some(option) => match option.value.as_ref() {
            Some(value) => match value.to_string().parse::<f32>() {
                Ok(value) => value,
                Err(e) => {
                    eprintln!("Failed to parce argument `volume` with error {e}");
                    crate::util::respond_to_interaction(
                        interaction,
                        &http,
                        true,
                        "Failed to parse argument `volume`",
                    )
                    .await;
                    return;
                }
            },
            None => {
                eprintln!("Failed to take reference to argument `volume`");
                crate::util::respond_to_interaction(
                    interaction,
                    &http,
                    true,
                    "Failed to parse argument `volume`",
                )
                .await;
                return;
            }
        },
        None => {
            crate::util::respond_to_interaction(
                interaction,
                &http,
                true,
                "Missing required argument `volume`",
            )
            .await;
            return;
        }
    };

    let guild_id = match interaction.guild_id {
        Some(id) => id.to_string(),
        None => {
            crate::util::respond_to_interaction(
                interaction,
                &http,
                true,
                "This command can only be used in a guild",
            )
            .await;
            return;
        }
    };
}

pub fn register(command: &mut CreateApplicationCommand) -> &mut CreateApplicationCommand {
    command
        .name("setvolume")
        .description("Sets the volume of the bot")
        .create_option(|option| {
            option
                .name("volume")
                .description("The volume of the rain")
                .kind(CommandOptionType::Number)
                .required(true)
        })
}
