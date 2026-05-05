# Coder

A Rust AI-powered development tool integrating features from Claude Code and OpenCode.

## OpenCode Free Tier

Coder comes with built-in support for OpenCode's free AI models via the Zen API proxy.

### Quick Start

1. Run `coder` for the first time
2. If no API key is found, a setup dialog will appear
3. Select **"Use OpenCode Free Tier"** to start immediately with anonymous access
4. Start coding — no API key required!

### Getting a Free API Key

For higher rate limits and access to more models:

1. Visit [https://opencode.ai/zen](https://opencode.ai/zen)
2. Create an account and generate an API key
3. Run `coder` and select **"Enter API Key Manually"** from the setup dialog
4. Or select **"Get Free API Key (OAuth)"** to authenticate via browser

### Configuration

Add to your `~/.coder/config.toml` or `coder.toml`:

```toml
[ai]
default_provider = "opencode"

[ai.providers.opencode]
provider_type = "opencode"
# api_key = "opk_..."  # Optional — leave empty for anonymous free tier
model = "claude-sonnet-4-6"
```

### Features

| Feature | Anonymous Mode | API Key Mode |
|---------|---------------|--------------|
| API Key Required | No | Yes |
| Rate Limiting | IP-based (100 req/min) | Workspace-based |
| Model Access | Limited free models | Full workspace models |
| Usage Tracking | Per-IP | Per-workspace |
| Cost | Free | Free (with rate limits) |

### Commands

- `/model` — Show current provider and model info
- `/model <name>` — Switch to a different model
