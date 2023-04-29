mod commands;
mod git;

use std::{fs, path::PathBuf, process::exit, sync::Mutex};

use commands::lexicon::{load, Lexicon};
use git::setup;
use serde::Deserialize;
use serenity::{
    async_trait,
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

pub struct Response {
    success: bool,
    title: String,
    text: String,
}

impl Response {
    pub fn success(title: impl Into<String>, text: impl Into<String>) -> Self {
        Self {
            success: true,
            title: title.into(),
            text: text.into(),
        }
    }

    pub fn failure(title: impl Into<String>, text: impl Into<String>) -> Self {
        Self {
            success: false,
            title: title.into(),
            text: text.into(),
        }
    }

    pub fn invalid_command() -> Self {
        Self {
            success: false,
            title: "Internal error".to_string(),
            text: "The command is invalid.".to_string(),
        }
    }

    pub fn unimplemented() -> Self {
        Self {
            success: false,
            title: "Internal error".to_string(),
            text: "The command is not implemented.".to_string(),
        }
    }
}

pub struct Handler {
    config: Config,
    lexicon: Mutex<Lexicon>,
}

#[async_trait]
impl EventHandler for Handler {
    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        if let Interaction::ApplicationCommand(command) = interaction {
            println!("/{}", command.data.name);
            let content = match command.data.name.as_str() {
                "lexicon" => commands::lexicon::run(self, &command.data.options),
                "test" => commands::test::run(&command.data.options),
                _ => Response::unimplemented(),
            };
            if let Err(why) = command
                .create_interaction_response(&ctx.http, |response| {
                    response
                        .kind(InteractionResponseType::ChannelMessageWithSource)
                        .interaction_response_data(|message| {
                            message.ephemeral(true).embed(|embed| {
                                if content.success {
                                    embed.color(Color::from_rgb(0x4b, 0xb5, 0x43));
                                } else {
                                    embed.color(Color::from_rgb(0xcc, 0x00, 0x00));
                                }
                                embed.title(&content.title).description(&content.text)
                            })
                        })
                })
                .await
            {
                eprintln!("Cannot respond to slash command: {}", why);
            }
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
        .event_handler(Handler {
            config,
            lexicon: Mutex::new(lexicon),
        })
        .await
        .expect("Error creating client");
    if let Err(err) = client.start().await {
        eprintln!("Client error: {err}");
    }
}
