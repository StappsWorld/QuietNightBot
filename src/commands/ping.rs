use serenity::{
    builder::CreateApplicationCommand, client::Context,
    model::prelude::interaction::application_command::ApplicationCommandInteraction,
};

pub async fn run(ctx: &Context, interaction: &ApplicationCommandInteraction) {
    crate::util::respond_to_interaction(
        interaction,
        &ctx.http,
        true,
        format!(
            "Pong! Bot time is <t:{}:F>",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs()
        ),
    )
    .await;
}

pub fn register(command: &mut CreateApplicationCommand) -> &mut CreateApplicationCommand {
    command.name("ping").description("A ping command")
}
