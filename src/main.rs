mod commands;
mod git;

use std::{fs, process::exit};

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
    Client,
};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    discord_token: String,
    git_username: String,
    git_email: String,
    git_password: String,
    git_url: String,
    git_path: String,
}

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        if let Interaction::ApplicationCommand(command) = interaction {
            println!("Received command interaction");
            let content = match command.data.name.as_str() {
                "test" => commands::test::run(&command.data.options),
                _ => "not implemented :(".to_string(),
            };
            if let Err(why) = command
                .create_interaction_response(&ctx.http, |response| {
                    response
                        .kind(InteractionResponseType::ChannelMessageWithSource)
                        .interaction_response_data(|message| message.content(content))
                })
                .await
            {
                println!("Cannot respond to slash command: {}", why);
            }
        }
    }

    async fn ready(&self, ctx: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name);
        let _test_command = Command::create_global_application_command(&ctx.http, |command| {
            commands::test::register(command)
        })
        .await
        .expect("Create global slash command(s)");
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
    setup(&config);
    let mut client = Client::builder(&config.discord_token, GatewayIntents::empty())
        .event_handler(Handler)
        .await
        .expect("Error creating client");
    if let Err(err) = client.start().await {
        eprintln!("Client error: {err}");
    }
}
