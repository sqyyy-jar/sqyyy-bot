use serenity::{
    builder::CreateApplicationCommand,
    model::prelude::interaction::application_command::CommandDataOption,
};

use crate::Response;

pub fn run(_options: &[CommandDataOption]) -> Response {
    Response::success("Test", "Hello world!")
}

pub fn register(command: &mut CreateApplicationCommand) -> &mut CreateApplicationCommand {
    command.name("test").description("Tests the bot")
}
