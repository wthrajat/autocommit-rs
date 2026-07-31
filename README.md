# `autocommit-rs`

[![crates.io](https://img.shields.io/crates/v/autocommit-rs.svg)](https://crates.io/crates/autocommit-rs)
[![CI](https://github.com/wthrajat/autocommit-rs/actions/workflows/ci.yml/badge.svg)](https://github.com/wthrajat/autocommit-rs/actions/workflows/ci.yml)

Generates and creates [conventional commits](https://www.conventionalcommits.org/en/v1.0.0/#specification) from staged changes in one go. Rust port of [@wthrajat/autocommit](https://github.com/wthrajat/autocommit).

![Demo](./public/assets/autocommit-demo.gif)


## Quick start

**Install**
```bash
cargo install autocommit-rs
```

**Update**
```bash
cargo install autocommit-rs --force
```

**Use** — stage changes, then run `autocommit` and choose: **Accept and commit**, **Edit message**, **Regenerate**, or **Quit**.

## Prerequisites

- [Rust](https://www.rust-lang.org/tools/install)
- [Gemini](https://aistudio.google.com/app/apikey) or [OpenAI](https://platform.openai.com/api-keys) API key

## CLI reference

| Flag | Description |
|------|-------------|
| `-v`, `--version` | Show version |
| `-h`, `--help` | Show help |
| `--openai-key <key>` | Set OpenAI API key |
| `--gemini-key <key>` | Set Gemini API key |
| `--model <model>` | Default model (`openai` or `gemini`) |
| `--short` / `--long` | Message style |
| `--sign` / `--no-sign` | GPG signing |
| `--no-verify` | Skip git hooks |
| `--stats` | Show context size, latency, and token usage |
| `--no-cache` | Disable the local generation cache for this run |

| Env variable | Description |
|--------------|-------------|
| `OPENAI_API_KEY` | Overrides config file |
| `GEMINI_API_KEY` | Overrides config file |
| `AUTOCOMMIT_MODEL` | `openai` or `gemini` |
| `AUTOCOMMIT_MESSAGE_STYLE` | `short` or `long` |
| `AUTOCOMMIT_OPENAI_MODEL` | Override the default `gpt-5-nano` model |
| `AUTOCOMMIT_GEMINI_MODEL` | Override the default `gemini-3.5-flash-lite` model |
| `AUTOCOMMIT_CACHE_PATH` | Override the local cache file path |

## Configuration

Stored in `~/.autocommitrc` (JSON). On first run, `autocommit` walks you through an interactive setup.

## Cost and latency behavior

- The staged patch is streamed into a balanced 10 KB context; generated files, lockfiles, and oversized lines cannot consume the whole prompt.
- Three alternatives are generated in one API request. **Regenerate** cycles through them before making another request.
- Results for an identical staged patch, model, prompt version, branch, and style are cached for 24 hours in `~/.autocommit-cache.json` with user-only permissions on Unix.
- API errors are reported instead of being replaced by a misleading fallback commit.
- `--stats` reports sizes, latency, and provider token counts without logging API keys or diff contents.

## Building from source

```bash
git clone https://github.com/wthrajat/autocommit-rs && cd autocommit-rs
cargo build --release --locked
cp ./target/release/autocommit ~/.local/bin/
```

## Local development

```bash
cargo build
cargo test
cargo run -- --help
```
