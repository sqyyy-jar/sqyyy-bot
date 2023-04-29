use serenity::{
    builder::CreateApplicationCommand,
    json::Value,
    model::prelude::{
        command::CommandOptionType, interaction::application_command::CommandDataOption,
    },
};

use crate::{Config, Response};

pub fn run(_config: &Config, options: &[CommandDataOption]) -> Response {
    dbg!(options);
    if options.len() != 1 {
        return Response::invalid_command();
    }
    let option = &options[0];
    match option.name.as_str() {
        "add" => {
            let options = &option.options;
            if option.options.len() != 2 {
                return Response::invalid_command();
            }
            let word = &options[0];
            let Some(Value::String(word)) = &word.value else {
                return Response::invalid_command();
            };
            let description = &options[1];
            let Some(Value::String(description)) = &description.value else {
                return Response::invalid_command();
            };
            todo!()
        }
        "query" => {
            let options = &option.options;
            if option.options.len() != 1 {
                return Response::invalid_command();
            }
            let word = &options[0];
            let Some(Value::String(word)) = &word.value else {
                return Response::invalid_command();
            };
            todo!()
        }
        "update" => {
            let options = &option.options;
            if option.options.len() != 2 {
                return Response::invalid_command();
            }
            let word = &options[0];
            let Some(Value::String(word)) = &word.value else {
                return Response::invalid_command();
            };
            let description = &options[1];
            let Some(Value::String(description)) = &description.value else {
                return Response::invalid_command();
            };
            todo!()
        }
        _ => Response::unimplemented(),
    }
}

pub fn register(command: &mut CreateApplicationCommand) -> &mut CreateApplicationCommand {
    command
        .name("lexicon")
        .description("Interacts with the lexicon")
        .create_option(|option| {
            option
                .name("add")
                .description("Add a lexicon entry")
                .kind(CommandOptionType::SubCommand)
                .create_sub_option(|option| {
                    option
                        .name("word")
                        .description("The word")
                        .kind(CommandOptionType::String)
                        .required(true)
                        .max_length(50)
                })
                .create_sub_option(|option| {
                    option
                        .name("description")
                        .description("The description")
                        .kind(CommandOptionType::String)
                        .required(true)
                        .max_length(6000)
                })
        })
        .create_option(|option| {
            option
                .name("query")
                .description("Query a lexicon entry")
                .kind(CommandOptionType::SubCommand)
                .create_sub_option(|option| {
                    option
                        .name("word")
                        .description("The word")
                        .kind(CommandOptionType::String)
                        .required(true)
                        .max_length(50)
                })
        })
        .create_option(|option| {
            option
                .name("update")
                .description("Update a lexicon entry")
                .kind(CommandOptionType::SubCommand)
                .create_sub_option(|option| {
                    option
                        .name("word")
                        .description("The word")
                        .kind(CommandOptionType::String)
                        .required(true)
                        .max_length(50)
                })
                .create_sub_option(|option| {
                    option
                        .name("description")
                        .description("The new description")
                        .kind(CommandOptionType::String)
                        .required(true)
                        .max_length(6000)
                })
        })
}
