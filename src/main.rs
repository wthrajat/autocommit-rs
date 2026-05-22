use anyhow::Result;
use clap::Parser;
use colored::Colorize;

mod ai;
mod classifier;
mod cli;
mod config;
mod git;
mod types;

async fn validate_git_state() -> Result<()> {
    git::is_git_repository()?;
    let has_changes = git::has_staged_changes()?;
    if !has_changes {
        cli::logger_warn("No staged changes found. Did you forget to run `git add`?");
        std::process::exit(0);
    }
    Ok(())
}

async fn generate_message(
    model: types::ModelType,
    message_style: types::MessageStyle,
) -> Result<String> {
    let (diff, files, branch_name) = tokio::try_join!(
        tokio::task::spawn_blocking(git::get_staged_diff),
        tokio::task::spawn_blocking(git::get_changed_files),
        tokio::task::spawn_blocking(git::get_branch_name),
    )?;
    let diff = diff?;
    let files = files?;
    let branch_name = branch_name?;

    let commit_type = classifier::classify_diff(&files, &diff);

    let spinner = cli::create_spinner("Analyzing diff and generating commit message...");
    let model_name = match model {
        types::ModelType::Gemini => "Gemini",
        types::ModelType::Openai => "OpenAI",
    };
    println!("{}", format!("Using {} for generation", model_name).blue());

    let message = ai::generate_commit_message(
        model,
        ai::GenerateOptions {
            diff: &diff,
            commit_type,
            files: &files,
            branch_name: &branch_name,
            message_style,
        },
    )
    .await;

    match message {
        Ok(msg) => {
            spinner.finish_with_message("Commit message generated!".to_string());
            Ok(msg)
        }
        Err(e) => {
            spinner.finish_with_message("Failed to generate message".to_string());
            cli::logger_error(&e.to_string());
            std::process::exit(1);
        }
    }
}

async fn get_user_action(message: &str) -> Result<(types::ActionType, String)> {
    let mut action = types::ActionType::Regenerate;
    let mut final_message = message.to_string();

    while action == types::ActionType::Regenerate {
        action = cli::ui::show_commit_options(&final_message)?;

        match action {
            types::ActionType::Accept => break,
            types::ActionType::Edit => match cli::ui::open_editor(&final_message) {
                Ok(edited) => {
                    final_message = edited;
                    break;
                }
                Err(e) => {
                    cli::logger_error(&format!("Failed to open editor: {}", e));
                    std::process::exit(1);
                }
            },
            types::ActionType::Quit => {
                cli::logger_info("Aborted.");
                std::process::exit(0);
            }
            types::ActionType::Regenerate => {}
        }
    }

    Ok((action, final_message))
}

async fn commit(message: &str, signed: bool, no_verify: bool) -> Result<()> {
    let spinner = cli::create_spinner("Committing...");
    let result = git::commit_changes(message, signed, no_verify);
    match result {
        Ok(()) => {
            spinner.finish_with_message("Committed successfully!".to_string());
            Ok(())
        }
        Err(e) => {
            spinner.finish_with_message("Git commit failed".to_string());
            cli::logger_error(&e.to_string());
            std::process::exit(1);
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();

    let args = cli::Args::parse();

    if let Some(key) = &args.openai_key {
        config::save_api_key(key, types::ModelType::Openai)?;
        cli::logger_success("OpenAI API key saved to ~/.autocommitrc!");
        return Ok(());
    }

    if let Some(key) = &args.gemini_key {
        config::save_api_key(key, types::ModelType::Gemini)?;
        cli::logger_success("Gemini API key saved to ~/.autocommitrc!");
        return Ok(());
    }

    if let Some(model_str) = &args.model {
        let model = match model_str.as_str() {
            "openai" => types::ModelType::Openai,
            "gemini" => types::ModelType::Gemini,
            other => {
                cli::logger_error(&format!(
                    "Please specify --model with \"openai\" or \"gemini\" (got: {})",
                    other
                ));
                std::process::exit(1);
            }
        };
        config::set_model(model)?;
        cli::logger_success(&format!("Default model set to {}!", model.as_str()));
        return Ok(());
    }

    if args.short {
        config::set_message_style(types::MessageStyle::Short)?;
        cli::logger_success("Message style set to short!");
        return Ok(());
    }

    if args.long {
        config::set_message_style(types::MessageStyle::Long)?;
        cli::logger_success("Message style set to long!");
        return Ok(());
    }

    if args.sign {
        config::set_signed_commit(true)?;
        cli::logger_success("Signed commits enabled!");
        return Ok(());
    }

    if args.no_sign {
        config::set_signed_commit(false)?;
        cli::logger_success("Signed commits disabled!");
        return Ok(());
    }

    if !config::config_file_exists() {
        cli::setup::run_interactive_setup()?;
    }

    let cfg = config::get_config()?;

    // SAFETY: Single-threaded CLI app; env var modification is safe here.
    unsafe {
        if cfg.model == types::ModelType::Gemini && !cfg.gemini_key.is_empty() {
            std::env::set_var("GEMINI_API_KEY", &cfg.gemini_key);
        } else if !cfg.openai_key.is_empty() {
            std::env::set_var("OPENAI_API_KEY", &cfg.openai_key);
        } else if !cfg.gemini_key.is_empty() {
            std::env::set_var("GEMINI_API_KEY", &cfg.gemini_key);
        }
    }

    let model = cfg.model;
    let message_style = config::get_message_style(&cfg);
    let signed = config::get_signed_commit(&cfg);
    let no_verify = args.no_verify;

    validate_git_state().await?;

    let message = generate_message(model, message_style).await?;

    let (_action, final_message) = get_user_action(&message).await?;

    if !final_message.is_empty() {
        commit(&final_message, signed, no_verify).await?;
    }

    Ok(())
}
