use std::{
    collections::BTreeMap,
    fs::{self, write, File},
    io::Write,
    process::exit,
};

use serde::{Deserialize, Serialize};
use serenity::{
    builder::CreateApplicationCommand,
    json::Value,
    model::prelude::{
        command::CommandOptionType, interaction::application_command::CommandDataOption,
    },
};

use crate::{git, Config, Handler, Response};

#[derive(Default, Deserialize, Serialize)]
pub struct Lexicon {
    words: BTreeMap<char, BTreeMap<String, String>>,
}

pub fn load(config: &Config) -> Lexicon {
    let mut path = config.git.path.clone();
    path.push(&config.lexicon.file);
    if !path.exists() || !path.is_file() {
        let file = File::create(&path);
        if let Err(err) = file {
            eprintln!("Could not create lexicon config: {err}");
            exit(1);
        }
        let mut file = file.unwrap();
        let lexicon = ron::to_string(&Lexicon::default());
        if let Err(err) = lexicon {
            eprintln!("Could not create lexicon config: {err}");
            exit(1);
        }
        if let Err(err) = file.write_all(lexicon.unwrap().as_bytes()) {
            eprintln!("Could not create lexicon config: {err}");
            exit(1);
        }
    }
    let lexicon = fs::read_to_string(&path);
    if let Err(err) = lexicon {
        eprintln!("Could not load lexicon config: {err}");
        exit(1);
    }
    let lexicon = lexicon.unwrap();
    let lexicon = ron::from_str(&lexicon);
    if let Err(err) = lexicon {
        eprintln!("Could not load lexicon config: {err}");
        exit(1);
    }
    lexicon.unwrap()
}

pub fn run(handler: &Handler, options: &[CommandDataOption]) -> Response {
    if options.len() != 1 {
        return Response::invalid_command();
    }
    let Ok(mut lexicon) = handler.lexicon.lock() else {
        return Response::failure("Internal error", "Could not lock the lexicon");
    };
    let option = &options[0];
    match option.name.as_str() {
        "add" => {
            let options = &option.options;
            if option.options.len() != 2 {
                return Response::invalid_command();
            }
            let Some(Value::String(word)) = &options[0].value else {
                return Response::invalid_command();
            };
            let Some(Value::String(description)) = &options[1].value else {
                return Response::invalid_command();
            };
            let c = word.chars().next().unwrap().to_uppercase().next().unwrap();
            let word_set = lexicon.words.entry(c).or_insert_with(BTreeMap::new);
            if word_set.contains_key(word) {
                return Response::failure("Query error", "The word already exists in the lexicon.");
            }
            word_set.insert(word.clone(), description.clone());
            if !update_lexicon(&handler.config, &lexicon) {
                return Response::failure("Update error", "The lexicon could not be updated.");
            }
            if !update_lexicon_git(&handler.config, &format!("[lexicon] Add {word}")) {
                return Response::failure("Git error", "The lexicon could not be pushed to Git.");
            }
            Response::success(
                "Success",
                format!("Successfully updated lexicon entry for word '{word}'."),
            )
        }
        "query" => {
            let options = &option.options;
            if option.options.len() != 1 {
                return Response::invalid_command();
            }
            let Some(Value::String(word)) = &options[0].value else {
                return Response::invalid_command();
            };
            let c = word.chars().next().unwrap().to_uppercase().next().unwrap();
            let Some(word_set) = lexicon.words.get(&c) else {
                return Response::failure("Query error", "The word could not be found.");
            };
            let Some(entry) = word_set.get(word) else {
                return Response::failure("Query error", "The word could not be found.");
            };
            Response::success(word, entry)
        }
        "update" => {
            let options = &option.options;
            if option.options.len() != 2 {
                return Response::invalid_command();
            }
            let Some(Value::String(word)) = &options[0].value else {
                return Response::invalid_command();
            };
            let Some(Value::String(description)) = &options[1].value else {
                return Response::invalid_command();
            };
            let c = word.chars().next().unwrap().to_uppercase().next().unwrap();
            let Some(word_set) = lexicon.words.get_mut(&c) else {
                return Response::failure("Query error", "The word could not be found.");
            };
            let Some(entry) = word_set.get_mut(word) else {
                return Response::failure("Query error", "The word could not be found.");
            };
            *entry = description.clone();
            if !update_lexicon(&handler.config, &lexicon) {
                return Response::failure("Update error", "The lexicon could not be updated.");
            }
            if !update_lexicon_git(&handler.config, &format!("[lexicon] Update {word}")) {
                return Response::failure("Git error", "The lexicon could not be pushed to Git.");
            }
            Response::success(
                "Success",
                format!("Successfully updated lexicon entry for word '{word}'."),
            )
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
                        .min_length(2)
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
                        .min_length(2)
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
                        .min_length(2)
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

fn update_lexicon(config: &Config, lexicon: &Lexicon) -> bool {
    let mut path = config.git.path.clone();
    path.push(&config.lexicon.file);
    let Ok(ser) = ron::to_string(lexicon) else {
        return false;
    };
    if write(path, ser).is_err() {
        return false;
    }
    let mut path = config.git.path.clone();
    path.push(&config.lexicon.target_file);
    let Ok(mut file) = File::create(path) else {
        return false;
    };
    if file.write_all(b"# Lexikon\n").is_err() {
        return false;
    }
    for (c, word_set) in &lexicon.words {
        if file.write_all(format!("\n## {c}\n").as_bytes()).is_err() {
            return false;
        }
        for (word, desc) in word_set {
            if file.write_all(b"\n### ").is_err() {
                return false;
            }
            if file.write_all(word.as_bytes()).is_err() {
                return false;
            }
            if file.write_all(b"\n\n").is_err() {
                return false;
            }
            if file.write_all(desc.as_bytes()).is_err() {
                return false;
            }
            if file.write_all(b"\n").is_err() {
                return false;
            }
        }
    }
    true
}

fn update_lexicon_git(config: &Config, message: &str) -> bool {
    if !git::stage(&config.git, config.lexicon.file.to_string_lossy().as_ref()) {
        return false;
    }
    if !git::stage(
        &config.git,
        config.lexicon.target_file.to_string_lossy().as_ref(),
    ) {
        return false;
    }
    if !git::commit(&config.git, message) {
        return false;
    }
    git::push(&config.git)
}
