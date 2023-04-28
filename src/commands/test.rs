use serenity::{model::prelude::interaction::application_command::CommandDataOption, builder::CreateApplicationCommand};

pub fn run(_options: &[CommandDataOption]) -> String {
    "Hey!".to_string()
}

pub fn register(command: &mut CreateApplicationCommand) -> &mut CreateApplicationCommand {
    command.name("test").description("Tests the bot")
}