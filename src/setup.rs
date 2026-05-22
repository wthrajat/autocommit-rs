use anyhow::Result;
use colored::Colorize;
use dialoguer::{Confirm, Password, Select};

use crate::config::{save_api_key, set_message_style, set_signed_commit};
use crate::types::{MessageStyle, ModelType};

pub fn run_interactive_setup() -> Result<()> {
    println!(
        "{}",
        "Welcome to autocommit! Let's set up your configuration.\n".yellow()
    );

    let model_selection = Select::new()
        .with_prompt("Which AI model would you like to use?")
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

    let api_key = Password::new()
        .with_prompt(format!("Enter your {} API key:", model_name))
        .validate_with(|input: &String| -> Result<(), &str> {
            if input.is_empty() {
                Err("API key is required")
            } else {
                Ok(())
            }
        })
        .interact()?;

    let style_selection = Select::new()
        .with_prompt("What commit message style do you prefer?")
        .items(&["Short (one-line summary)", "Long (with description)"])
        .default(0)
        .interact()?;

    let message_style = match style_selection {
        0 => MessageStyle::Short,
        _ => MessageStyle::Long,
    };

    let signed_commit = Confirm::new()
        .with_prompt("Sign commits with GPG?")
        .default(false)
        .interact()?;

    save_api_key(&api_key, model)?;
    set_message_style(message_style)?;
    set_signed_commit(signed_commit)?;

    print_setup_success();
    Ok(())
}

fn print_setup_success() {
    println!("{} Configuration saved to ~/.autocommitrc!", "✔".green());
    println!(
        "{} You can change these settings anytime with:",
        " ".dimmed()
    );
    println!("  autocommit --openai-key \"key\"");
    println!("  autocommit --gemini-key \"key\"");
    println!("  autocommit --model openai|gemini");
    println!("  autocommit --short|--long\n");
}
