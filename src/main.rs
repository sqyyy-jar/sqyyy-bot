mod commands;
mod git;
mod parser;

use std::{collections::HashMap, fs, mem, path::PathBuf, process::exit, sync::Mutex};

use commands::lexicon::{create_add_modal, create_update_modal, load, Lexicon};
use git::setup;
use serde::Deserialize;
use serenity::{
    async_trait,
    builder::{CreateComponents, CreateInteractionResponse, CreateInteractionResponseData},
    model::prelude::{
        command::Command,
        interaction::{Interaction, InteractionResponseType},
        *,
    },
    prelude::*,
    utils::Color,
    Client,
};

#[derive(Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Config {
    discord: DiscordConfig,
    git: GitConfig,
    lexicons: Vec<LexiconConfig>,
}

#[derive(Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct DiscordConfig {
    token: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct GitConfig {
    username: String,
    email: String,
    password: String,
    url: String,
    path: PathBuf,
}

#[derive(Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct LexiconConfig {
    name: String,
    file: PathBuf,
    target_file: PathBuf,
}

pub enum Response {
    Regular {
        success: bool,
        title: String,
        text: String,
    },
    Modal {
        creation: fn(&Handler, &mut CreateInteractionResponseData),
        modal: Modal,
    },
}

impl Response {
    pub fn success(title: impl Into<String>, text: impl Into<String>) -> Self {
        Self::Regular {
            success: true,
            title: title.into(),
            text: text.into(),
        }
    }

    pub fn failure(title: impl Into<String>, text: impl Into<String>) -> Self {
        Self::Regular {
            success: false,
            title: title.into(),
            text: text.into(),
        }
    }

    pub fn invalid_command() -> Self {
        Self::Regular {
            success: false,
            title: "Internal error".to_string(),
            text: "The command is invalid.".to_string(),
        }
    }

    pub fn unimplemented() -> Self {
        Self::Regular {
            success: false,
            title: "Internal error".to_string(),
            text: "The command is not implemented.".to_string(),
        }
    }

    pub fn modal(creation: fn(&Handler, &mut CreateInteractionResponseData), modal: Modal) -> Self {
        Self::Modal { creation, modal }
    }

    pub fn handle(self, handler: &Handler, response: &mut CreateInteractionResponse<'_>) {
        match self {
            Response::Regular {
                success,
                title,
                text,
            } => {
                response
                    .kind(InteractionResponseType::ChannelMessageWithSource)
                    .interaction_response_data(|message| {
                        message.embed(|embed| {
                            embed.title(title).description(text).color(if success {
                                Color::from_rgb(0x4b, 0xb5, 0x43)
                            } else {
                                Color::from_rgb(0xcc, 0x00, 0x00)
                            })
                        })
                    });
            }
            Response::Modal { creation, modal } => {
                let id = match modal {
                    Modal::Test => "test".to_string(),
                    Modal::LexiconAdd { .. } => {
                        let mut modals = handler.modals.lock().unwrap();
                        let id = modals.0;
                        modals.1.insert(id, modal);
                        modals.0 += 1;
                        id.to_string()
                    }
                    Modal::LexiconUpdate { .. } => {
                        let mut modals = handler.modals.lock().unwrap();
                        let id = modals.0;
                        modals.1.insert(id, modal);
                        modals.0 += 1;
                        id.to_string()
                    }
                    Modal::Emulate => "emulate".to_string(),
                };
                response
                    .kind(InteractionResponseType::Modal)
                    .interaction_response_data(|message| {
                        creation(handler, message);
                        message.custom_id(id)
                    });
            }
        }
    }
}

pub enum Modal {
    Test,
    LexiconAdd { index: usize },
    LexiconUpdate { index: usize },
    Emulate,
}

pub struct Handler {
    config: Config,
    lexicons: Vec<Mutex<Lexicon>>,
    modals: Mutex<(u64, HashMap<u64, Modal>)>,
    lexicon_add_modal: CreateComponents,
    lexicon_update_modal: CreateComponents,
}

impl Handler {
    pub fn load(mut config: Config) -> Self {
        if config.lexicons.is_empty() {
            eprintln!("No lexicons present");
            exit(1);
        }
        if config.lexicons.len() > 25 {
            eprintln!("Too many lexicons (max. 25)");
            exit(1);
        }
        let mut lexicons = Vec::with_capacity(config.lexicons.len());
        let lexicon_configs = mem::take(&mut config.lexicons);
        for lexicon_config in lexicon_configs {
            let lexicon = load(&config, lexicon_config);
            lexicons.push(Mutex::new(lexicon));
        }
        let lexicon_add_modal = create_add_modal();
        let lexicon_update_modal = create_update_modal();
        Self {
            config,
            modals: Mutex::new((0, HashMap::new())),
            lexicons,
            lexicon_add_modal,
            lexicon_update_modal,
        }
    }
}

#[async_trait]
impl EventHandler for Handler {
    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        match interaction {
            Interaction::ApplicationCommand(command) => {
                if command.guild_id.is_none() {
                    println!("{} (DM) /{} - blocked", command.user, command.data.name);
                    return;
                }
                println!("{} /{}", command.user, command.data.name);
                let content = match command.data.name.as_str() {
                    "lexicon" => commands::lexicon::run(self, &command.user, &command.data.options),
                    "emulate" => commands::emulate::run(&command.data.options),
                    "test" => commands::test::run(&command.data.options),
                    _ => Response::unimplemented(),
                };
                if let Err(why) = command
                    .create_interaction_response(&ctx.http, move |response| {
                        content.handle(self, response);
                        response
                    })
                    .await
                {
                    eprintln!("Cannot respond to slash command: {}", why);
                }
            }
            Interaction::ModalSubmit(mut submission) => {
                if submission.guild_id.is_none() {
                    println!(
                        "{} (DM) modal:{} - blocked",
                        submission.user, submission.data.custom_id
                    );
                    return;
                }
                let custom_id = submission.data.custom_id.as_str();
                let content = match custom_id {
                    "test" => commands::test::handle_modal(&mut submission).await,
                    "emulate" => commands::emulate::handle_modal(&mut submission).await,
                    _ => {
                        let Ok(custom_id) = submission.data.custom_id.parse() else {
                            eprintln!("Cannot parse modal submission");
                            return;
                        };
                        let modal = {
                            let mut modals = self.modals.lock().unwrap();
                            modals.1.remove(&custom_id)
                        };
                        let Some(modal) = modal else {
                            return;
                        };
                        match modal {
                            Modal::Test => commands::test::handle_modal(&mut submission).await,
                            Modal::LexiconAdd { index } => {
                                commands::lexicon::handle_add(self, index, &mut submission).await
                            }
                            Modal::LexiconUpdate { index } => {
                                commands::lexicon::handle_update(self, index, &mut submission).await
                            }
                            Modal::Emulate => {
                                commands::emulate::handle_modal(&mut submission).await
                            }
                        }
                    }
                };
                if let Err(why) = submission
                    .create_interaction_response(&ctx.http, |response| {
                        content.handle(self, response);
                        response
                    })
                    .await
                {
                    eprintln!("Cannot respond to slash command: {}", why);
                }
            }
            _ => {}
        }
    }

    async fn ready(&self, ctx: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name);
        Command::create_global_application_command(&ctx.http, |command| {
            commands::lexicon::register(self, command)
        })
        .await
        .expect("Create lexicon command");
        Command::create_global_application_command(&ctx.http, |command| {
            commands::emulate::register(command)
        })
        .await
        .expect("Create emulate command");
        Command::create_global_application_command(&ctx.http, |command| {
            commands::test::register(command)
        })
        .await
        .expect("Create test command");
    }
}

#[tokio::main]
async fn main() {
    let config_src = fs::read_to_string(".config.toml");
    if let Err(err) = config_src {
        eprintln!("Could not read config file: {err}");
        exit(1);
    }
    let config = toml::from_str(&config_src.unwrap());
    if let Err(err) = config {
        eprintln!("Could not parse config file: {err}");
        exit(1);
    }
    let config: Config = config.unwrap();
    setup(&config.git);
    let mut client = Client::builder(&config.discord.token, GatewayIntents::empty())
        .event_handler(Handler::load(config))
        .await
        .expect("Error creating client");
    if let Err(err) = client.start().await {
        eprintln!("Client error: {err}");
    }
}
