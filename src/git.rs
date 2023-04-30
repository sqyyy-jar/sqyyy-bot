use std::{
    fs,
    process::{exit, Command, Stdio},
};

use crate::GitConfig;

fn run(config: &GitConfig, args: &[&str]) -> bool {
    Command::new("git")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .args(args)
        .current_dir(&config.path)
        .status()
        .expect("Execute git command")
        .success()
}

pub fn setup(config: &GitConfig) {
    if let Err(err) = fs::create_dir_all(&config.path) {
        eprintln!("Could not create repository directory: {err}");
        exit(1);
    }
    if !run(config, &["status"]) {
        clone(config);
        login(config);
    }
    if !run(config, &["status"]) {
        eprintln!("Could not setup git repository");
        exit(1);
    }
}

fn clone(config: &GitConfig) {
    if !run(
        config,
        &[
            "clone",
            &format!(
                "https://{}:{}@{}",
                config.username,
                config.password,
                config.url.trim_start_matches("https://")
            ),
            ".",
        ],
    ) {
        eprintln!("Could not clone git repository");
        exit(1);
    }
}

fn login(config: &GitConfig) {
    if !run(config, &["config", "user.name", &config.username]) {
        eprintln!("Could not set git username");
        exit(1);
    }
    if !run(config, &["config", "user.email", &config.email]) {
        eprintln!("Could not set git email");
        exit(1);
    }
    if !run(config, &["config", "commit.gpgsign", "false"]) {
        eprintln!("Could not disable GPG signing");
        exit(1);
    }
}

pub fn stage(config: &GitConfig, what: &str) -> bool {
    run(config, &["add", what])
}

pub fn commit(config: &GitConfig, message: &str) -> bool {
    run(config, &["commit", "-m", message])
}

pub fn pull(config: &GitConfig) -> bool {
    run(config, &["pull"])
}

pub fn push(config: &GitConfig) -> bool {
    run(config, &["push"])
}
