# AI providers setup

i can generate commit messages for you with _technology!_ O.O

## what AI providers do

i can use AI to analyze your staged changes and generate commit messages:

```bash
git add .
cocoa generate
```

messages are generated based on:

1. your staged changes
2. your branch name
3. recent commits for context

then i'll show you the generated commit message for you to approve or refine!

**want to write messages yourself, but with a fancy TUI?** heck yeah! use interactive mode instead
with `cocoa commit` :D

## supported providers

through the `genai` crate, i can chat with these providers:

- OpenAI ChatGPT
- Anthropic Claude
- Ollama
- OpenRouter

you may want to check the `genai` crate for a complete and up-to-date list.

## OpenAI ChatGPT

### 1. get an API key

go to the [OpenAI API Console](https://platform.openai.com/api-keys) and generate an API key there.

**store it securely** and **don't commit it to git**, you silly goose!

### 2. tell me to talk to ChatGPT

in `.cocoa.toml`:

```toml
[ai]
provider = "openai"
model = "gpt-4"
temperature = 0.7
max_tokens = 500

[ai.secret]
env = "OPENAI_API_KEY"
```

### 3. set the environment variable

**on macOS/Linux:**

```bash
# temporarily (current session only)
export OPENAI_API_KEY="sk-your-key-here"

# permanently (add to ~/.zshrc or ~/.bashrc)
echo 'export OPENAI_API_KEY="sk-your-key-here"' >> ~/.zshrc
source ~/.zshrc
```

**on Windows:**

```bash
# PowerShell
$env:OPENAI_API_KEY="sk-your-key-here"

# Command Prompt
setx OPENAI_API_KEY "sk-your-key-here"
```

### 4. test!

```bash
git add .
cocoa generate
```

## Anthropic Claude

### 1. get an API key

go to the [Claude Platform Console](https://platform.claude.com/) and generate an API key there.

**store it securely** and **don't commit it to git**, you silly goose!

### 2. tell me talk to Claude

in `.cocoa.toml`:

```toml
[ai]
provider = "anthropic"
model = "claude-4-opus"
temperature = 0.7
max_tokens = 500

[ai.secret]
env = "ANTHROPIC_API_KEY"
```

### 3. set the environment variable

**on macOS/Linux:**

```bash
export ANTHROPIC_API_KEY="sk-ant-your-key-here"
echo 'export ANTHROPIC_API_KEY="sk-ant-your-key-here"' >> ~/.zshrc
```

**on Windows:**

```bash
setx ANTHROPIC_API_KEY "sk-ant-your-key-here"
```

### 4. test!

```bash
git add .
cocoa generate
```

## Ollama (Free, Local)

you can run an AI model locally on your own machine!

### 1. install Ollama

go to [Ollama.ai](https://ollama.ai) for instructions and downloads.

### 2. download a model

you only have to do this once for any model you want to try:

```bash
ollama pull <model>
```

you can check out Ollama's website for their [list of models](https://ollama.com/models).

then, just run the Ollama server:

```bash
# Start Ollama server
ollama serve
```

Ollama runs at `http://localhost:11434` by default.

### 3. tell me to talk to the llama

hooved friends really go well together, don't they?

in `.cocoa.toml`:

```toml
[ai]
provider = "ollama"
model = "mistral"
temperature = 0.7
max_tokens = 500

[ai.secret]
env = "OLLAMA_API_KEY"    # Can be blank or dummy value
```

there's no API key needed (it's local!), but the env field is required:

```bash
export OLLAMA_API_KEY=""
cocoa generate
```

## OpenRouter (multi-provider)

### 1. get an API Key

1. go to [OpenRouter.ai](https://openrouter.ai)
2. sign up
3. get an API key
4. copy it (starts with `sk-or-`)

### 2. set me up

in `.cocoa.toml`:

```toml
[ai]
provider = "openrouter"
model = "openai/gpt-4"           # OpenAI models
# model = "anthropic/claude-opus"  # Anthropic
# model = "mistralai/mistral-7b"  # Mistral

temperature = 0.7
max_tokens = 500

[ai.secret]
env = "OPENROUTER_API_KEY"
```

### 3. Set Environment Variable

```bash
export OPENROUTER_API_KEY="sk-or-your-key-here"
```

### 4. Available Models

check [OpenRouter Models](https://openrouter.ai/models) for full list:

```toml
[ai]
# OpenAI
model = "openai/gpt-4"
model = "openai/gpt-3.5-turbo"

# Anthropic
model = "anthropic/claude-3-opus"
model = "anthropic/claude-3-sonnet"

# Mistral
model = "mistralai/mistral-7b"
model = "mistralai/mistral-medium"

# Meta
model = "meta-llama/llama-2-70b-chat"
```

## secure API key management

### option 1: environment variables

```bash
export OPENAI_API_KEY="sk-..."
cocoa generate
```

### option 2: from file

```toml
[ai.secret]
file = "~/.openai_key"       # Read from file
# or
file = "/run/secrets/openai"  # Docker secrets
```

create the file:

```bash
echo "sk-your-key" > ~/.openai_key
chmod 600 ~/.openai_key      # Only readable by you
```

good for docker/containers, CI/CD, or server deployments

### option 3: GitHub Actions secret

```yaml
# .github/workflows/release.yml
jobs:
  release:
    runs-on: ubuntu-latest
    env:
      OPENAI_API_KEY: ${{ secrets.OPENAI_API_KEY }}
    steps:
      - run: cocoa generate
```

## troubleshooting

### "API key not found"

if you want to use environment variables, are they set correctly?

```bash
echo $OPENAI_API_KEY
# should print your key

# if blank, set it:
export OPENAI_API_KEY="sk-..."
```

### "invalid API key"

1. copy the full key (sometimes copy/paste cuts it off!)
2. make sure it hasn't expired
3. check you're using the right provider

### "model not found"

**check available models:**

- OpenAI: [https://platform.openai.com/docs/models](https://platform.openai.com/docs/models)
- Anthropic: [https://docs.anthropic.com/models](https://docs.anthropic.com/models)
- Ollama: [https://ollama.com/models](https://ollama.com/models)

### "rate limited"

wait and retry later. most providers reset limits hourly/daily.

sometimes they just need a break from me :(

### "connection timeout"

1. is your internet connection working?
2. is the API provider's service up? (check their status page)
3. if you're using Ollama, is the server running? (`ollama serve`)

## team setup

### managed service

use a secret manager for CI/CD:

- GitHub Actions: [secrets](https://docs.github.com/en/actions/security-guides/encrypted-secrets)
- GitLab CI: [variables](https://docs.gitlab.com/ee/ci/variables/)
- AWS: [secrets manager](https://aws.amazon.com/secrets-manager/)
