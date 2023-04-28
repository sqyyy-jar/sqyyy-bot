use std::{
    fs,
    process::{exit, Command, Stdio},
};

use crate::Config;

fn run(config: &Config, args: &[&str]) -> bool {
    Command::new("git")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .args(args)
        .current_dir(&config.git_path)
        .status()
        .expect("Execute git command")
        .success()
}

pub fn setup(config: &Config) {
    if let Err(err) = fs::create_dir_all(&config.git_path) {
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

fn clone(config: &Config) {
    if !run(
        config,
        &[
            "clone",
            &format!(
                "https://{}:{}@{}",
                config.git_username,
                config.git_password,
                config.git_url.trim_start_matches("https://")
            ),
            ".",
        ],
    ) {
        eprintln!("Could not clone git repository");
        exit(1);
    }
}

fn login(config: &Config) {
    if !run(config, &["config", "user.name", &config.git_username]) {
        eprintln!("Could not set git username");
        exit(1);
    }
    if !run(config, &["config", "user.email", &config.git_email]) {
        eprintln!("Could not set git email");
        exit(1);
    }
    if !run(config, &["config", "commit.gpgsign", "false"]) {
        eprintln!("Could not disable GPG signing");
        exit(1);
    }
}

pub fn stage(config: &Config, what: &str) -> bool {
    run(config, &["add", what])
}

pub fn commit(config: &Config, message: &str) -> bool {
    run(config, &["commit", "-m", message])
}

pub fn push(config: &Config) -> bool {
    run(config, &["push"])
}
