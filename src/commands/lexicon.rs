use std::{
    collections::BTreeMap,
    fs::{self, write, File},
    io::Write,
    path::PathBuf,
    process::exit,
};

use serde::{Deserialize, Serialize};
use serenity::{
    builder::{CreateApplicationCommand, CreateComponents},
    json::Value,
    model::{
        prelude::{
            command::CommandOptionType,
            component::{ActionRowComponent, InputTextStyle},
            interaction::{application_command::CommandDataOption, modal::ModalSubmitInteraction},
        },
        user::User,
    },
};

use crate::{git, Config, Handler, LexiconConfig, Modal, Response};

#[derive(Default, Deserialize, Serialize)]
pub struct Lexicon {
    name: String,
    file: PathBuf,
    target_file: PathBuf,
    words: BTreeMap<char, BTreeMap<String, String>>,
}

pub fn load(config: &Config, lexicon_config: LexiconConfig) -> Lexicon {
    let mut path = config.git.path.clone();
    path.push(&lexicon_config.file);
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
    let words_str = lexicon.unwrap();
    let words = ron::from_str(&words_str);
    if let Err(err) = words {
        eprintln!("Could not load lexicon config: {err}");
        exit(1);
    }
    Lexicon {
        name: lexicon_config.name,
        file: lexicon_config.file,
        target_file: lexicon_config.target_file,
        words: words.unwrap(),
    }
}

pub fn run(handler: &Handler, user: &User, options: &[CommandDataOption]) -> Response {
    if options.len() != 1 {
        return Response::invalid_command();
    }
    let option = &options[0];
    match option.name.as_str() {
        "add" => {
            let options = &option.options;
            if option.options.len() != 1 {
                return Response::invalid_command();
            }
            let Some(Value::String(lexicon_name)) = &options[0].value else {
                return Response::invalid_command();
            };
            let mut index = None;
            for (i, lexicon_) in handler.lexicons.iter().enumerate() {
                let guard = lexicon_.lock().unwrap();
                if &guard.name == lexicon_name {
                    index = Some(i);
                    break;
                }
            }
            let Some(index) = index else {
                return Response::failure("Add entry error", "The lexicon could not be found.");
            };
            Response::modal(
                |handler, response| {
                    response
                        .title("Add lexicon entry")
                        .set_components(handler.lexicon_add_modal.clone());
                },
                Modal::LexiconAdd { index },
            )
        }
        "query" => {
            let options = &option.options;
            if option.options.len() != 2 {
                return Response::invalid_command();
            }
            let Some(Value::String(lexicon_name)) = &options[0].value else {
                return Response::invalid_command();
            };
            let mut lexicon = None;
            for lexicon_ in &handler.lexicons {
                let guard = lexicon_.lock().unwrap();
                if &guard.name == lexicon_name {
                    lexicon = Some(guard);
                    break;
                }
            }
            let Some(lexicon) = lexicon else {
                return Response::failure("Query entry error", "The lexicon could not be found.");
            };
            let Some(Value::String(word)) = &options[1].value else {
                return Response::invalid_command();
            };
            let c = word.chars().next().unwrap().to_uppercase().next().unwrap();
            let Some(word_set) = lexicon.words.get(&c) else {
                return Response::failure("Query entry error", "The word could not be found.");
            };
            let Some(entry) = word_set.get(word) else {
                return Response::failure("Query entry error", "The word could not be found.");
            };
            Response::success(word, entry)
        }
        "update" => {
            let options = &option.options;
            if option.options.len() != 1 {
                return Response::invalid_command();
            }
            let Some(Value::String(lexicon_name)) = &options[0].value else {
                return Response::invalid_command();
            };
            let mut index = None;
            for (i, lexicon_) in handler.lexicons.iter().enumerate() {
                let guard = lexicon_.lock().unwrap();
                if &guard.name == lexicon_name {
                    index = Some(i);
                    break;
                }
            }
            let Some(index) = index else {
                return Response::failure("Update entry error", "The lexicon could not be found.");
            };
            Response::modal(
                |handler, response| {
                    response
                        .title("Update lexicon entry")
                        .set_components(handler.lexicon_update_modal.clone());
                },
                Modal::LexiconUpdate { index },
            )
        }
        "remove" => {
            let options = &option.options;
            if option.options.len() != 2 {
                return Response::invalid_command();
            }
            let Some(Value::String(lexicon_name)) = &options[0].value else {
                return Response::invalid_command();
            };
            let mut lexicon = None;
            for lexicon_ in &handler.lexicons {
                let guard = lexicon_.lock().unwrap();
                if &guard.name == lexicon_name {
                    lexicon = Some(guard);
                    break;
                }
            }
            let Some(mut lexicon) = lexicon else {
                return Response::failure("Remove entry error", "The lexicon could not be found.");
            };
            let Some(Value::String(word)) = &options[1].value else {
                return Response::invalid_command();
            };
            let c = word.chars().next().unwrap().to_uppercase().next().unwrap();
            let Some(word_set) = lexicon.words.get_mut(&c) else {
                return Response::failure("Remove entry error", "The word could not be found.");
            };
            if word_set.remove(word).is_none() {
                return Response::failure("Remove entry error", "The word could not be found.");
            }
            if word_set.is_empty() {
                lexicon.words.remove(&c);
            }
            if !update_lexicon(&handler.config, &lexicon) {
                return Response::failure("Update error", "The lexicon could not be updated.");
            }
            if !update_lexicon_git(
                &handler.config,
                &lexicon,
                &format!("[lexicon] Remove {word} - {}", user.name),
            ) {
                return Response::failure("Git error", "The lexicon could not be pushed to Git.");
            }
            Response::success("Success", "The word got removed.")
        }
        _ => Response::unimplemented(),
    }
}

pub async fn handle_add(
    handler: &Handler,
    index: usize,
    submission: &mut ModalSubmitInteraction,
) -> Response {
    let mut rows = submission.data.components.drain(..);
    let mut first = rows.next().unwrap().components.into_iter();
    let mut second = rows.next().unwrap().components.into_iter();
    let word = first.next().unwrap();
    let ActionRowComponent::InputText(word) = word else {unreachable!()};
    let word = word.value;
    let description = second.next().unwrap();
    let ActionRowComponent::InputText(description) = description else {unreachable!()};
    let description = description.value;
    let c = word.chars().next().unwrap().to_uppercase().next().unwrap();
    let mut lexicon = handler.lexicons[index].lock().unwrap();
    let word_set = lexicon.words.entry(c).or_insert_with(BTreeMap::new);
    if word_set.contains_key(&word) {
        return Response::failure("Add entry error", "The word already exists in the lexicon.");
    }
    word_set.insert(word.clone(), description);
    if !update_lexicon(&handler.config, &lexicon) {
        return Response::failure("Update error", "The lexicon could not be updated.");
    }
    if !update_lexicon_git(
        &handler.config,
        &lexicon,
        &format!("[lexicon] Add {word} - {}", submission.user.name),
    ) {
        return Response::failure("Git error", "The lexicon could not be pushed to Git.");
    }
    Response::success(
        "Success",
        format!("Successfully updated lexicon entry for word '{word}'."),
    )
}

pub async fn handle_update(
    handler: &Handler,
    index: usize,
    submission: &mut ModalSubmitInteraction,
) -> Response {
    let mut rows = submission.data.components.drain(..);
    let mut first = rows.next().unwrap().components.into_iter();
    let mut second = rows.next().unwrap().components.into_iter();
    let word = first.next().unwrap();
    let ActionRowComponent::InputText(word) = word else {unreachable!()};
    let word = word.value;
    let description = second.next().unwrap();
    let ActionRowComponent::InputText(description) = description else {unreachable!()};
    let description = description.value;
    let c = word.chars().next().unwrap().to_uppercase().next().unwrap();
    let mut lexicon = handler.lexicons[index].lock().unwrap();
    let Some(word_set) = lexicon.words.get_mut(&c) else {
        return Response::failure("Update entry error", "The word could not be found.");
    };
    let Some(entry) = word_set.get_mut(&word) else {
        return Response::failure("Update entry error", "The word could not be found.");
    };
    if entry == &description {
        return Response::success("Success", "Nothing changed.");
    }
    *entry = description;
    if !update_lexicon(&handler.config, &lexicon) {
        return Response::failure("Update error", "The lexicon could not be updated.");
    }
    if !update_lexicon_git(
        &handler.config,
        &lexicon,
        &format!("[lexicon] Update {word} - {}", submission.user.name),
    ) {
        return Response::failure("Git error", "The lexicon could not be pushed to Git.");
    }
    Response::success(
        "Success",
        format!("Successfully updated lexicon entry for word '{word}'."),
    )
}

pub fn register<'a>(
    handler: &Handler,
    command: &'a mut CreateApplicationCommand,
) -> &'a mut CreateApplicationCommand {
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
                        .name("lexicon")
                        .description("The lexicon")
                        .kind(CommandOptionType::String)
                        .required(true)
                        .min_length(1)
                        .max_length(50);
                    for lexicon in &handler.lexicons {
                        let lexicon = lexicon.lock().unwrap();
                        option.add_string_choice(&lexicon.name, &lexicon.name);
                    }
                    option
                })
        })
        .create_option(|option| {
            option
                .name("query")
                .description("Query a lexicon entry")
                .kind(CommandOptionType::SubCommand)
                .create_sub_option(|option| {
                    option
                        .name("lexicon")
                        .description("The lexicon")
                        .kind(CommandOptionType::String)
                        .required(true)
                        .min_length(1)
                        .max_length(50);
                    for lexicon in &handler.lexicons {
                        let lexicon = lexicon.lock().unwrap();
                        option.add_string_choice(&lexicon.name, &lexicon.name);
                    }
                    option
                })
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
                        .name("lexicon")
                        .description("The lexicon")
                        .kind(CommandOptionType::String)
                        .required(true)
                        .min_length(1)
                        .max_length(50);
                    for lexicon in &handler.lexicons {
                        let lexicon = lexicon.lock().unwrap();
                        option.add_string_choice(&lexicon.name, &lexicon.name);
                    }
                    option
                })
        })
        .create_option(|option| {
            option
                .name("remove")
                .description("Remove a lexicon entry")
                .kind(CommandOptionType::SubCommand)
                .create_sub_option(|option| {
                    option
                        .name("lexicon")
                        .description("The lexicon")
                        .kind(CommandOptionType::String)
                        .required(true)
                        .min_length(1)
                        .max_length(50);
                    for lexicon in &handler.lexicons {
                        let lexicon = lexicon.lock().unwrap();
                        option.add_string_choice(&lexicon.name, &lexicon.name);
                    }
                    option
                })
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
}

fn update_lexicon(config: &Config, lexicon: &Lexicon) -> bool {
    let mut path = config.git.path.clone();
    path.push(&lexicon.file);
    let Ok(ser) = ron::to_string(&lexicon.words) else {
        return false;
    };
    if write(path, ser).is_err() {
        return false;
    }
    let mut path = config.git.path.clone();
    path.push(&lexicon.target_file);
    let Ok(mut file) = File::create(path) else {
        return false;
    };
    if file
        .write_all(b"<!--THIS FILE IS AUTOMATICALLY GENERATED - DO NOT EDIT-->\n")
        .is_err()
    {
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

fn update_lexicon_git(config: &Config, lexicon: &Lexicon, message: &str) -> bool {
    if !git::stage(&config.git, lexicon.file.to_string_lossy().as_ref()) {
        return false;
    }
    if !git::stage(&config.git, lexicon.target_file.to_string_lossy().as_ref()) {
        return false;
    }
    if !git::commit(&config.git, message) {
        return false;
    }
    git::push(&config.git)
}

pub fn create_add_modal() -> CreateComponents {
    let mut components = CreateComponents::default();
    components
        .create_action_row(|row| {
            row.create_input_text(|input| {
                input
                    .label("Word")
                    .custom_id("modal:lexicon:add:0")
                    .style(InputTextStyle::Short)
                    .required(true)
                    .min_length(2)
                    .max_length(50)
            })
        })
        .create_action_row(|row| {
            row.create_input_text(|input| {
                input
                    .label("Description")
                    .custom_id("modal:lexicon:add:1")
                    .style(InputTextStyle::Paragraph)
                    .required(true)
                    .max_length(4000)
            })
        });
    components
}

pub fn create_update_modal() -> CreateComponents {
    let mut components = CreateComponents::default();
    components
        .create_action_row(|row| {
            row.create_input_text(|input| {
                input
                    .label("Word")
                    .custom_id("modal:lexicon:update:0")
                    .style(InputTextStyle::Short)
                    .required(true)
                    .min_length(2)
                    .max_length(50)
            })
        })
        .create_action_row(|row| {
            row.create_input_text(|input| {
                input
                    .label("Description")
                    .custom_id("modal:lexicon:update:1")
                    .style(InputTextStyle::Paragraph)
                    .required(true)
                    .max_length(4000)
            })
        });
    components
}
