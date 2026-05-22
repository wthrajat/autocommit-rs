# `autocommit` (Rust port)

Tool that generates and pushes [conventional commits](https://www.conventionalcommits.org/en/v1.0.0/#specification) from staged changes in one go. For free :)

This is a Rust port of the original [@wthrajat/autocommit](https://github.com/wthrajat/autocommit) TypeScript project.

## Prerequisites

- [Rust](https://www.rust-lang.org/tools/install) (edition 2024)
- One of these API keys:
  - [Gemini](https://aistudio.google.com/app/apikey) (free tier available)
  - [OpenAI](https://platform.openai.com/api-keys)

## Installation

### From source

```bash
git clone <repo-url>
cd autocommit/rust-autocommit
cargo build --release
```

The binary will be at `./target/release/autocommit`. You can copy it to your PATH:

```bash
cp ./target/release/autocommit ~/.local/bin/
# or
cargo install --path .
```

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
cd rust-autocommit
cargo build
cargo test
cargo run -- --help
```

## License

MIT
