use serenity::{
    builder::CreateApplicationCommand,
    model::prelude::{
        component::{ActionRowComponent, InputTextStyle},
        interaction::{application_command::CommandDataOption, modal::ModalSubmitInteraction},
    },
};

use crate::{Modal, Response};

pub fn run(_options: &[CommandDataOption]) -> Response {
    Response::modal(
        |_, msg| {
            msg.title("Test modal");
            msg.components(|components| {
                components.create_action_row(|row| {
                    row.create_input_text(|input| {
                        input
                            .label("Test input")
                            .custom_id(0)
                            .style(InputTextStyle::Paragraph)
                            .required(true)
                            .max_length(4000)
                    })
                })
            });
        },
        Modal::Test,
    )
}

pub async fn handle_modal(submission: &mut ModalSubmitInteraction) -> Response {
    let ActionRowComponent::InputText(input) = submission
        .data
        .components
        .drain(..)
        .next()
        .unwrap()
        .components
        .drain(..)
        .next()
        .unwrap() else {unimplemented!()};
    Response::success("Test response", input.value)
}

pub fn register(command: &mut CreateApplicationCommand) -> &mut CreateApplicationCommand {
    command.name("test").description("Tests the bot")
}
