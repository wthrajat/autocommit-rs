use clap::Parser;
use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle};

pub mod setup;
pub mod ui;

/// Tool that generates and pushes conventional commits from staged changes in one go.
#[derive(Parser, Debug)]
#[command(name = "autocommit")]
#[command(version)]
#[command(about, long_about = None)]
pub struct Args {
    /// Set OpenAI API key
    #[arg(long, value_name = "KEY")]
    pub openai_key: Option<String>,

    /// Set Gemini API key
    #[arg(long, value_name = "KEY")]
    pub gemini_key: Option<String>,

    /// Set default model (openai or gemini)
    #[arg(long, value_name = "MODEL")]
    pub model: Option<String>,

    /// Use short message style
    #[arg(long)]
    pub short: bool,

    /// Use long message style
    #[arg(long)]
    pub long: bool,

    /// Enable GPG signed commits
    #[arg(long)]
    pub sign: bool,

    /// Disable GPG signed commits
    #[arg(long)]
    pub no_sign: bool,

    /// Bypass pre-commit and commit-msg git hooks
    #[arg(long)]
    pub no_verify: bool,
}

pub fn logger_info(msg: &str) {
    println!("{} {}", "ℹ".blue(), msg);
}

pub fn logger_success(msg: &str) {
    println!("{} {}", "✔".green(), msg);
}

pub fn logger_warn(msg: &str) {
    println!("{} {}", "⚠".yellow(), msg);
}

pub fn logger_error(msg: &str) {
    eprintln!("{} {}", "✖".red(), msg);
}

pub fn create_spinner(text: &str) -> ProgressBar {
    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::default_spinner()
            .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏")
            .template("{spinner} {msg}")
            .unwrap(),
    );
    spinner.set_message(text.to_string());
    spinner.enable_steady_tick(std::time::Duration::from_millis(80));
    spinner
}
