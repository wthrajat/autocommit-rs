use anyhow::Result;
use colored::Colorize;
use dialoguer::{Confirm, Password, Select};

use super::ui::prompt_theme;
use crate::config::{Config, save_config};
use crate::types::{MessageStyle, ModelType};

pub fn run_interactive_setup() -> Result<()> {
    super::print_app_header("A quick setup, then your staged changes stay in flow");
    let theme = prompt_theme();

    let model_selection = Select::with_theme(&theme)
        .with_prompt("AI provider")
        .items(&["OpenAI", "Google Gemini"])
        .default(0)
        .interact()?;

    let model = match model_selection {
        0 => ModelType::Openai,
        _ => ModelType::Gemini,
    };

    let model_name = match model {
        ModelType::Openai => "OpenAI",
        ModelType::Gemini => "Gemini",
    };

    let api_key = Password::with_theme(&theme)
        .with_prompt(format!("{model_name} API key"))
        .validate_with(|input: &String| -> Result<(), &str> {
            if input.trim().is_empty() {
                Err("API key is required")
            } else {
                Ok(())
            }
        })
        .interact()?
        .trim()
        .to_string();

    let style_selection = Select::with_theme(&theme)
        .with_prompt("Commit message style")
        .items(&["Short · one-line summary", "Long · summary with bullets"])
        .default(0)
        .interact()?;

    let message_style = match style_selection {
        0 => MessageStyle::Short,
        _ => MessageStyle::Long,
    };

    let signed_commit = Confirm::with_theme(&theme)
        .with_prompt("Sign commits with GPG?")
        .default(false)
        .interact()?;

    let mut config = Config {
        model,
        message_style,
        signed_commit,
        ..Config::default()
    };
    match model {
        ModelType::Openai => config.openai_key = api_key,
        ModelType::Gemini => config.gemini_key = api_key,
    }
    save_config(&config)?;

    print_setup_success(model_name, message_style, signed_commit);
    Ok(())
}

fn print_setup_success(model_name: &str, message_style: MessageStyle, signed_commit: bool) {
    let signing = if signed_commit { "GPG on" } else { "GPG off" };
    println!();
    super::logger_success("Setup complete");
    println!(
        "  {}  {} · {} messages · {signing}",
        "Saved".dimmed(),
        model_name,
        message_style.as_str(),
    );
    println!(
        "  {}",
        "Run `autocommit --help` to change settings later.".dimmed()
    );
    println!();
}
