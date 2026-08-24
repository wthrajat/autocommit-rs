use clap::Parser;
use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle};

pub mod setup;
pub mod ui;

/// Generate and create conventional commits from staged changes in one go.
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

    /// Show context size, latency, and token usage
    #[arg(long)]
    pub stats: bool,

    /// Do not read or write the local generation cache
    #[arg(long)]
    pub no_cache: bool,
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

pub fn print_error_chain(error: &anyhow::Error) {
    let mut causes = error.chain();
    let Some(root) = causes.next() else {
        return;
    };
    println!("{} {}", "✖".red(), root.to_string().bold().red());
    for cause in causes {
        println!("   {} {}", "└".dimmed(), cause.to_string().dimmed());
    }
}

pub fn print_app_header(context: &str) {
    let context = ui::sanitize_terminal_text(context).replace('\n', " ");
    let version = concat!("v", env!("CARGO_PKG_VERSION"));
    println!();
    println!(
        "{} {} {}",
        "◆".cyan(),
        "autocommit".bold(),
        version.bright_black()
    );
    println!("  {}", context.dimmed());
    println!();
}

pub fn stat_line(label: &str, value: &str) {
    println!("  {}  {}", format!("{label:<7}").bright_black(), value);
}

pub fn create_spinner(text: &str) -> ProgressBar {
    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::default_spinner()
            .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏ ")
            .template("  {spinner:.cyan} {msg}")
            .unwrap(),
    );
    spinner.set_message(text.to_string());
    spinner.enable_steady_tick(std::time::Duration::from_millis(80));
    spinner
}
