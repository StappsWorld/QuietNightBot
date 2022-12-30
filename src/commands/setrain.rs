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
    let on = match interaction.data.options.get(0) {
        Some(option) => match option.value.as_ref() {
            Some(value) => match value.to_string().parse::<bool>() {
                Ok(value) => value,
                Err(e) => {
                    eprintln!("Failed to parce argument `on` with error {e}");
                    crate::util::respond_to_interaction(
                        interaction,
                        &http,
                        true,
                        "Failed to parse argument `on`",
                    )
                    .await;
                    return;
                }
            },
            None => {
                eprintln!("Failed to take reference to argument `on`");
                crate::util::respond_to_interaction(
                    interaction,
                    &http,
                    true,
                    "Failed to parse argument `on`",
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
                "Missing required argument `on`",
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

    {
        let mut rain_enabled_hashmap = match timeout(
            std::time::Duration::from_secs(5),
            crate::util::RAIN_ENABLED.lock(),
        )
        .await
        {
            Ok(lock) => lock,
            Err(_) => {
                eprintln!("Failed to get lock on rain_enabled hashmap");
                crate::util::respond_to_interaction(
                    interaction,
                    &http,
                    true,
                    "There was an internal error. Please try again later",
                )
                .await;
                return;
            }
        };

        rain_enabled_hashmap.insert(guild_id, on);
    }

    crate::util::respond_to_interaction(
        interaction,
        &http,
        false,
        format!("User {} set rain to {}", interaction.user.tag(), on),
    )
    .await;
}

pub fn register(command: &mut CreateApplicationCommand) -> &mut CreateApplicationCommand {
    command
        .name("setrain")
        .description("Sets the rain effect")
        .create_option(|option| {
            option
                .name("on")
                .description("Turns the rain effect on/off")
                .kind(CommandOptionType::Boolean)
                .required(true)
        })
}
