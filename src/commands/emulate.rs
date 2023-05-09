use emulator::emulator::Emulator;
use serenity::{
    builder::CreateApplicationCommand,
    model::prelude::{
        component::{ActionRowComponent, InputTextStyle},
        interaction::{application_command::CommandDataOption, modal::ModalSubmitInteraction},
    },
};

use crate::{
    parser::{parse, tokenize},
    Modal, Response,
};

pub fn run(_options: &[CommandDataOption]) -> Response {
    Response::modal(
        |_, msg| {
            msg.title("Circuit emulator");
            msg.components(|components| {
                components.create_action_row(|row| {
                    row.create_input_text(|input| {
                        input
                            .label("Code")
                            .custom_id(0)
                            .style(InputTextStyle::Paragraph)
                            .required(true)
                            .max_length(4000)
                    })
                })
            });
        },
        Modal::Emulate,
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
    let mut response = String::new();
    for (i, statement) in input.value.split('\n').enumerate() {
        let tokens = tokenize(statement);
        if let Err(err) = tokens {
            return Response::failure(format!("[{i}] Parsing error"), err.to_string());
        }
        let tokens = tokens.unwrap();
        let component = parse(&tokens);
        if let Err(err) = component {
            return Response::failure(format!("[{i}] Parsing error"), err.to_string());
        }
        let (input_count, component) = component.unwrap();
        if input_count > 16 {
            // maximum count of inputs
            return Response::failure(format!("[{i}] Emulation error"), "Too many inputs");
        }
        let emulator = Emulator::new(input_count, component);
        if emulator.is_err() {
            return Response::failure(
                format!("[{i}] Emulation error"),
                "Could not create emulator",
            );
        }
        let emulator = emulator.unwrap();
        let emulation = emulator.emulate_all();
        if emulation.is_err() {
            return Response::failure(format!("[{i}] Emulation error"), "Could not emulate");
        }
        let emulation = emulation.unwrap();
        response.push_str(&format!("[{i}]:\n```\n"));
        response.push_str(&emulation.to_string());
        response.push_str("```\n");
    }
    Response::success("Success", response)
}

pub fn register(command: &mut CreateApplicationCommand) -> &mut CreateApplicationCommand {
    command.name("emulate").description("Emulate a circuit")
}
