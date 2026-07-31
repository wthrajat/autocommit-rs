use anyhow::Result;
use clap::Parser;
use colored::Colorize;

mod ai;
mod classifier;
mod cli;
mod config;
mod git;
mod types;

struct PreparedChanges {
    diff: git::diff::DiffContext,
    files: Vec<String>,
    branch_name: String,
    commit_type: Option<types::CommitType>,
}

fn prepare_changes() -> Result<Option<PreparedChanges>> {
    let repository = git::get_repository_state()?;
    if repository.staged_files.is_empty() {
        cli::logger_warn("No staged changes found. Did you forget to run `git add`?");
        return Ok(None);
    }

    let diff = git::get_staged_diff_context(&repository.staged_files)?;
    let commit_type = classifier::classify_files(&repository.staged_files);
    Ok(Some(PreparedChanges {
        diff,
        files: repository.staged_files,
        branch_name: repository.branch_name,
        commit_type,
    }))
}

async fn generate_candidates(
    generator: &ai::Generator,
    changes: &PreparedChanges,
    message_style: types::MessageStyle,
    excluded_candidates: &[String],
    use_cache: bool,
    show_stats: bool,
) -> Result<Vec<String>> {
    let spinner = cli::create_spinner("Generating commit message candidates...");
    let result = generator
        .generate(
            ai::GenerateOptions {
                diff: &changes.diff.patch,
                diff_fingerprint: &changes.diff.fingerprint,
                commit_type: changes.commit_type,
                files: &changes.files,
                branch_name: &changes.branch_name,
                message_style,
            },
            excluded_candidates,
            use_cache,
        )
        .await;

    match result {
        Ok(result) => {
            let message = if result.metrics.cache_hit {
                "Loaded commit messages from cache"
            } else {
                "Commit messages generated"
            };
            spinner.finish_and_clear();
            cli::logger_success(message);
            if show_stats {
                print_generation_stats(&result.metrics);
            }
            Ok(result.candidates)
        }
        Err(error) => {
            spinner.finish_and_clear();
            Err(error)
        }
    }
}

async fn choose_commit_message(
    generator: &ai::Generator,
    changes: &PreparedChanges,
    message_style: types::MessageStyle,
    use_cache: bool,
    show_stats: bool,
) -> Result<Option<String>> {
    let mut candidates = generate_candidates(
        generator,
        changes,
        message_style,
        &[],
        use_cache,
        show_stats,
    )
    .await?;
    let mut current_index = 0;

    loop {
        let current = &candidates[current_index];
        match cli::ui::show_commit_options(current)? {
            types::ActionType::Accept => return Ok(Some(current.clone())),
            types::ActionType::Edit => {
                return cli::ui::open_editor(current).map(Some);
            }
            types::ActionType::Quit => {
                cli::logger_info("Aborted.");
                return Ok(None);
            }
            types::ActionType::Regenerate => {
                current_index += 1;
                if current_index == candidates.len() {
                    let new_candidates = generate_candidates(
                        generator,
                        changes,
                        message_style,
                        &candidates,
                        use_cache,
                        show_stats,
                    )
                    .await?;
                    candidates.extend(new_candidates);
                }
            }
        }
    }
}

fn commit(message: &str, signed: bool, no_verify: bool) -> Result<()> {
    let spinner = cli::create_spinner("Committing...");
    match git::commit_changes(message, signed, no_verify) {
        Ok(()) => {
            spinner.finish_and_clear();
            cli::logger_success("Commit created");
            Ok(())
        }
        Err(error) => {
            spinner.finish_and_clear();
            Err(error)
        }
    }
}

fn print_context_stats(stats: &git::diff::DiffStats) {
    let truncation = if stats.truncated { " · truncated" } else { "" };
    println!(
        "  {}  {} → {} · {} generated patches omitted{}",
        "Context".dimmed(),
        format_bytes(stats.raw_bytes),
        format_bytes(stats.included_bytes),
        stats.omitted_generated_files,
        truncation,
    );
}

fn print_generation_stats(metrics: &ai::GenerationMetrics) {
    if metrics.cache_hit {
        println!(
            "  {}       {} · local cache · 0 tokens",
            "AI".dimmed(),
            metrics.model
        );
        return;
    }

    let usage = &metrics.usage;
    println!(
        "  {}       {} · {:.2}s · {} input · {} cached · {} output tokens",
        "AI".dimmed(),
        metrics.model,
        metrics.duration.as_secs_f64(),
        format_optional_count(usage.input_tokens),
        format_optional_count(usage.cached_input_tokens),
        format_optional_count(usage.output_tokens),
    );
}

fn format_optional_count(count: Option<u64>) -> String {
    count.map_or_else(|| "unknown".to_string(), |count| count.to_string())
}

fn format_bytes(bytes: usize) -> String {
    if bytes < 1_024 {
        format!("{bytes} B")
    } else {
        format!("{:.1} KiB", bytes as f64 / 1_024.0)
    }
}

#[tokio::main(flavor = "current_thread")]
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
                anyhow::bail!(
                    "Please specify --model with \"openai\" or \"gemini\" (got: {other})"
                );
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

    let Some(changes) = prepare_changes()? else {
        return Ok(());
    };
    let mut config = config::get_config()?;
    if !config.has_selected_api_key() {
        cli::setup::run_interactive_setup()?;
        config = config::get_config()?;
    }

    let model = config.model;
    let api_key = config.api_key_for(model).to_string();
    let generator = ai::Generator::new(model, api_key)?;
    let changed_files = changes.diff.stats.changed_files;
    let file_label = if changed_files == 1 {
        "staged file"
    } else {
        "staged files"
    };
    cli::print_app_header(&format!(
        "{} {file_label} · {} · {}",
        changed_files,
        generator.provider_name(),
        generator.model_name(),
    ));
    if args.stats {
        print_context_stats(&changes.diff.stats);
        println!();
    }

    let message = choose_commit_message(
        &generator,
        &changes,
        config.message_style,
        !args.no_cache,
        args.stats,
    )
    .await?;
    if let Some(message) = message.filter(|message| !message.is_empty()) {
        commit(&message, config.signed_commit, args.no_verify)?;
    }
    Ok(())
}
