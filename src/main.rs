mod commands;
mod git;

use std::{collections::HashMap, fs, path::PathBuf, process::exit, sync::Mutex};

use commands::lexicon::{load, Lexicon};
use git::setup;
use serde::Deserialize;
use serenity::{
    async_trait,
    builder::{CreateInteractionResponse, CreateInteractionResponseData},
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
    lexicon: LexiconConfig,
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
        creation: fn(&mut CreateInteractionResponseData),
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

    pub fn modal(creation: fn(&mut CreateInteractionResponseData), modal: Modal) -> Self {
        Self::Modal { creation, modal }
    }

    pub fn handle(
        self,
        response: &mut CreateInteractionResponse<'_>,
        modals: &Mutex<(u64, HashMap<u64, Modal>)>,
    ) {
        match self {
            Response::Regular {
                success,
                title,
                text,
            } => {
                response
                    .kind(InteractionResponseType::ChannelMessageWithSource)
                    .interaction_response_data(|message| {
                        message.ephemeral(true).embed(|embed| {
                            embed.title(title).description(text).color(if success {
                                Color::from_rgb(0x4b, 0xb5, 0x43)
                            } else {
                                Color::from_rgb(0xcc, 0x00, 0x00)
                            })
                        })
                    });
            }
            Response::Modal { creation, modal } => {
                let mut modals = modals.lock().unwrap();
                let id = modals.0;
                response
                    .kind(InteractionResponseType::Modal)
                    .interaction_response_data(|message| {
                        creation(message);
                        message.custom_id(id)
                    });
                modals.1.insert(id, modal);
                modals.0 += 1;
            }
        }
    }
}

pub enum Modal {
    Test,
    LexiconAdd,
    LexiconUpdate,
}

pub struct Handler {
    config: Config,
    lexicon: Mutex<Lexicon>,
    modals: Mutex<(u64, HashMap<u64, Modal>)>,
}

impl Handler {
    pub fn new(config: Config, lexicon: Lexicon) -> Self {
        Self {
            config,
            lexicon: Mutex::new(lexicon),
            modals: Mutex::new((0, HashMap::new())),
        }
    }
}

#[async_trait]
impl EventHandler for Handler {
    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        match interaction {
            Interaction::ApplicationCommand(command) => {
                println!("{} /{}", command.user.mention(), command.data.name);
                let content = match command.data.name.as_str() {
                    "lexicon" => commands::lexicon::run(self, &command.data.options),
                    "test" => commands::test::run(&command.data.options),
                    _ => Response::unimplemented(),
                };
                if let Err(why) = command
                    .create_interaction_response(&ctx.http, move |response| {
                        content.handle(response, &self.modals);
                        response
                    })
                    .await
                {
                    eprintln!("Cannot respond to slash command: {}", why);
                }
            }
            Interaction::ModalSubmit(mut submission) => {
                let custom_id: u64 = submission.data.custom_id.parse::<u64>().unwrap();
                let modal = {
                    let mut modals = self.modals.lock().unwrap();
                    let Some(modal) = modals.1.remove(&custom_id) else {
                        return;
                    };
                    modal
                };
                let content = match modal {
                    Modal::Test => commands::test::handle_modal(&mut submission).await,
                    Modal::LexiconAdd => commands::lexicon::handle_add(self, &mut submission).await,
                    Modal::LexiconUpdate => {
                        commands::lexicon::handle_update(self, &mut submission).await
                    }
                };
                if let Err(why) = submission
                    .create_interaction_response(&ctx.http, |response| {
                        content.handle(response, &self.modals);
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
            commands::lexicon::register(command)
        })
        .await
        .expect("Create lexicon command");
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
    let lexicon = load(&config);
    let mut client = Client::builder(&config.discord.token, GatewayIntents::empty())
        .event_handler(Handler::new(config, lexicon))
        .await
        .expect("Error creating client");
    if let Err(err) = client.start().await {
        eprintln!("Client error: {err}");
    }
}
