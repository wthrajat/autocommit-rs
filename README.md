# `autocommit-rs`

[![Crates.io](https://img.shields.io/crates/v/autocommit.svg)](https://crates.io/crates/autocommit)

Tool that generates and publishes [conventional commits](https://www.conventionalcommits.org/en/v1.0.0/#specification) from staged changes in one go. For free :)

This is a Rust port of the original [@wthrajat/autocommit](https://github.com/wthrajat/autocommit) TypeScript project.

## Prerequisites

- [Rust](https://www.rust-lang.org/tools/install) (for `cargo install`)
- One of these API keys:
  - [Gemini](https://aistudio.google.com/app/apikey) (free tier available)
  - [OpenAI](https://platform.openai.com/api-keys)

## Installation

### From crates.io (recommended)

```bash
cargo install autocommit
```

### From source

```bash
git clone https://github.com/wthrajat/autocommit
cd autocommit
cargo build --release
```

The binary will be at `./target/release/autocommit`. Copy it to your PATH:

```bash
cp ./target/release/autocommit ~/.local/bin/
```

### Pre-built binaries

Pre-built binaries for Linux, macOS (Intel & Apple Silicon), and Windows are available on the [GitHub Releases page](https://github.com/wthrajat/autocommit/releases).

## Updating

```bash
cargo install autocommit --force
```

Or download the latest pre-built binary from the [GitHub Releases page](https://github.com/wthrajat/autocommit/releases).

## Usage

1. Do code changes in any repo you're working on and stage them:
   ```bash
   git add <files>
   ```

2. Run `autocommit` in the terminal:
   ```bash
   autocommit
   ```

3. Choose an option:
   - **Accept and commit**: Commits right away.
   - **Edit message**: Opens a text editor to adjust the message before committing.
   - **Regenerate**: Asks the AI for a new attempt.
   - **Quit**: Cancels the operation.

### Command-line options

| Flag | Description |
|------|-------------|
| `-v`, `--version` | Show version |
| `-h`, `--help` | Show help message |
| `--openai-key <key>` | Set OpenAI API key |
| `--gemini-key <key>` | Set Gemini API key |
| `--model <model>` | Set default model (`openai` or `gemini`) |
| `--short` | Use short message style (one-line summary) |
| `--long` | Use long message style (with description) |
| `--sign` | Enable GPG signed commits |
| `--no-sign` | Disable GPG signed commits |
| `--no-verify` | Bypass pre-commit and commit-msg git hooks |

### Environment variables

| Variable | Description |
|----------|-------------|
| `OPENAI_API_KEY` | OpenAI API key (overrides config file) |
| `GEMINI_API_KEY` | Google Gemini API key (overrides config file) |
| `AUTOCOMMIT_MODEL` | Model to use: `openai` or `gemini` |
| `AUTOCOMMIT_MESSAGE_STYLE` | Message style: `short` or `long` |

## Configuration

Configuration is stored in `~/.autocommitrc` as JSON. On first run, `autocommit` will guide you through an interactive setup.

## Local development

```bash
cargo build
cargo test
cargo run -- --help
```

## Releasing

New releases are automated via GitHub Actions. To publish a new version:

### Initial setup (one-time)

1. **Log in to crates.io**
   ```bash
   cargo login
   ```
   Follow the prompt to create and paste an API token from [crates.io/tokens](https://crates.io/tokens).

2. **Add the token to GitHub Secrets**
   - Go to repo settings → Secrets and variables → Actions
   - Add `CRATES_IO_TOKEN` with the token from step 1

### Release a new version

1. **Update the version** in `Cargo.toml`:
   ```toml
   version = "0.2.0"
   ```

2. **Commit and push** to `main`:
   ```bash
   git add Cargo.toml
   git commit -m "chore(release): bump version to 0.2.0"
   git push origin main
   ```

The release workflow will automatically (no need to push tags manually):
- Create a git tag (`v0.2.0`)
- Publish the crate to [crates.io](https://crates.io/crates/autocommit)
- Build binaries for Linux, macOS, and Windows
- Create a [GitHub Release](https://github.com/wthrajat/autocommit/releases) with the binaries attached

### Manual publish (if needed)

If you need to publish without the automated workflow:

```bash
cargo publish
```

## License

MIT
