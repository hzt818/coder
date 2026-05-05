# OpenCode Zen API Integration Design

**Date:** 2026-05-05
**Status:** Draft
**Author:** Coder Team

---

## 1. Summary

Integrate OpenCode's Zen API as a built-in AI provider in the coder project, enabling users to access free AI models without requiring their own API keys. Supports both anonymous (IP-based rate-limited) access and authenticated (workspace API key) access.

---

## 2. Goals

- Add "opencode" as a first-class AI provider type in coder's provider system
- Support anonymous (no API key) usage with IP-based rate limiting
- Support authenticated usage with OpenCode workspace API key
- Display interactive startup dialog when no API key is configured
- Dynamically fetch available models from Zen API on startup
- Provide both OAuth and manual API key input methods
- Seamlessly integrate with coder's existing configuration and TUI

---

## 3. Architecture

```
coder startup
    │
    ├── load config
    ├── check for API keys
    │   ├── has key → normal startup
    │   └── no key → ProviderSetupDialog (TUI)
    │       ├── [Use OpenCode Free Tier] → anonymous mode
    │       ├── [Get API Key (OAuth)]   → OAuth flow → save key
    │       ├── [Enter API Key Manually] → manual input → save key
    │       └── [Configure Later]       → exit, user can /config later
    │
    └── create provider → fetch models → enter main loop
```

### 3.1 Provider Layer

```
ai::Provider (trait)
    ▲
    ├── openai::OpenAIProvider    (existing)
    ├── anthropic::AnthropicProvider (existing)
    └── opencode::OpenCodeProvider  (new)
            │
            ├── anonymous mode:  no auth header, IP-based rate limiting
            ├── key mode:        "Authorization: Bearer <api_key>"
            │
            └── reuses OpenAI-compatible SSE parsing from openai.rs
```

### 3.2 Module Dependencies

```
src/ai/opencode.rs
  └── uses openai.rs SSE parsing utilities (parse_sse_stream, process_sse_data)

src/tui/dialog_provider_setup.rs
  └── uses crossterm + ratatui for dialog rendering
  └── uses oauth2 crate for OAuth flow

src/config/mod.rs
  └── exports opencode provider config

src/main.rs
  └── startup check → dialog_provider_setup → config update
```

---

## 4. Detailed Design

### 4.1 OpenCodeProvider (`src/ai/opencode.rs`)

```rust
/// OpenCode Zen API provider
///
/// Acts as an OpenAI-compatible wrapper that:
/// - Defaults to anonymous mode (no API key required)
/// - Adds Bearer auth when API key is provided
/// - Fetches model list from /zen/v1/models at startup
pub struct OpenCodeProvider {
    api_key: Option<String>,   // None = anonymous mode
    base_url: String,          // default: https://opencode.ai/zen/v1
    model: String,
    available_models: Vec<String>,
}
```

**Key behaviors:**

| Mode | API Key | Auth Header | Rate Limiting | Model Access |
|------|---------|-------------|---------------|--------------|
| Anonymous | None | None | IP-based (100 req/min) | `allowAnonymous` models |
| Authenticated | `opk_...` | `Bearer <key>` | Key-based | Full workspace models |

**API endpoints used:**

| Endpoint | Method | Purpose |
|----------|--------|---------|
| `/zen/v1/models` | GET | Fetch available models |
| `/zen/v1/chat/completions` | POST | Chat completions (OpenAI-compat) |
| `/zen/v1/messages` | POST | Messages API (Anthropic-compat, future) |

### 4.2 Provider Registration (`src/ai/mod.rs`)

Add a new match arm in `create_provider()`:

```rust
"opencode" => {
    #[cfg(feature = "ai-opencode")]
    {
        let provider = OpenCodeProvider::new(
            config.api_key,   // None = anonymous
            config.base_url,  // default: https://opencode.ai/zen/v1
            model,
        );
        // Fetch available models from Zen API
        if let Ok(models) = provider.fetch_models().await {
            provider.set_available_models(models);
        }
        Ok(Box::new(provider))
    }
    #[cfg(not(feature = "ai-opencode"))]
    anyhow::bail!("OpenCode provider requires 'ai-opencode' feature")
}
```

### 4.3 Feature Flag

Add `Cargo.toml` feature:

```toml
[features]
ai-opencode = []
default = ["tui", "ai-openai", "ai-anthropic", "ai-opencode", "tools-core"]
```

### 4.4 Default Config (`src/config/settings.rs`)

```toml
[ai]
default_provider = "opencode"

[ai.providers.opencode]
provider_type = "opencode"
# api_key is optional — leave empty for anonymous/free tier
base_url = "https://opencode.ai/zen/v1"
model = "claude-sonnet-4-6"
```

### 4.5 Startup Provider Setup Dialog (`src/tui/dialog_provider_setup.rs`)

A TUI dialog rendered when no provider has an API key configured:

```
┌─────────────────────────────────────────────────┐
│  🔑 AI Provider Setup                           │
│                                                 │
│  No API key found. How would you like to        │
│  connect to an AI provider?                     │
│                                                 │
│  ┌─────────────────────────────────────────┐   │
│  │ ○ Use OpenCode Free Tier (anonymous)     │   │
│  │   Start immediately, IP-based rate limit │   │
│  │                                          │   │
│  │ ○ Get Free API Key (OAuth)              │   │
│  │   Sign in via browser, get API key      │   │
│  │                                          │   │
│  │ ○ Enter API Key Manually                │   │
│  │   Paste your API key from opencode.ai   │   │
│  │                                          │   │
│  │ ○ Skip — I'll configure later           │   │
│  │   Use /config command to set up later   │   │
│  └─────────────────────────────────────────┘   │
│                                                 │
│  [Select]                    [Quit]             │
└─────────────────────────────────────────────────┘
```

**States:**

| Option | Action | Resulting State |
|--------|--------|-----------------|
| Free Tier | Set provider=opencode, api_key=None | Write to config, continue |
| OAuth | Launch browser, OAuth callback | Save key to config, continue |
| Manual | Show text input field | Save key to config, continue |
| Skip | No changes | Continue without provider |

### 4.6 OAuth Flow

Reuse existing `oauth2` crate dependency:

```
User clicks "Get Free API Key (OAuth)"
    → coder starts local HTTP server on localhost:<port>
    → opens browser to https://opencode.ai/oauth/authorize?redirect_uri=http://localhost:<port>/callback
    → user signs in and authorizes
    → OpenCode redirects to localhost:<port>/callback?code=...
    → coder exchanges code for API key
    → saves API key to ~/.coder/config.toml
    → closes browser, continues startup
```

**File:** `src/oauth/opencode.rs` (or integrate into existing `src/oauth/`)

### 4.7 Model Fetching

```rust
impl OpenCodeProvider {
    pub async fn fetch_models(&self) -> anyhow::Result<Vec<String>> {
        let client = reqwest::Client::new();
        let mut req = client.get(format!("{}/models", self.base_url));
        
        if let Some(ref key) = self.api_key {
            req = req.header("Authorization", format!("Bearer {}", key));
        }
        
        let resp = req.send().await?;
        let data: ModelsResponse = resp.json().await?;
        
        // Extract model IDs from response
        Ok(data.data.into_iter().map(|m| m.id).collect())
    }
}
```

### 4.8 Chat Completion Flow

```
user sends message
    → OpenCodeProvider::chat_stream()
        → build OpenAI-compatible request body
        → POST /zen/v1/chat/completions
            → anonymous: no auth header
            → key mode: add Bearer auth header
        → parse SSE stream (reuse openai.rs SSE parser)
        → emit StreamEvent items
```

---

## 5. Error Handling

| Scenario | Error | User Experience |
|----------|-------|-----------------|
| Anonymous rate limit hit | `429 Too Many Requests` | "Free tier rate limit reached. Try again in a few minutes or get an API key." |
| Invalid API key | `401 Unauthorized` | "Invalid API key. Check your key at opencode.ai/zen" |
| Model unavailable | `404 Model Not Found` | "Model not available. Use /model to see available models." |
| Network error | Connection failed | "Cannot reach OpenCode API. Check your internet connection." |
| OAuth timeout | User didn't complete | "OAuth timed out. Try again or enter API key manually." |

---

## 6. Files to Create/Modify

### New files:

| File | Purpose |
|------|---------|
| `src/ai/opencode.rs` | OpenCode provider implementation |
| `src/tui/dialog_provider_setup.rs` | Startup provider selection dialog |
| `src/oauth/opencode.rs` | OpenCode OAuth flow |

### Modified files:

| File | Changes |
|------|---------|
| `Cargo.toml` | Add `ai-opencode` feature flag |
| `src/ai/mod.rs` | Add `opencode` module and match arm in `create_provider()` |
| `src/config/settings.rs` | Add OpenCode as default provider when no key configured |
| `src/main.rs` | Add startup API key check and dialog trigger |
| `src/ai/types.rs` | Possibly add OpenCode-specific types |
| `src/tui/app.rs` | May need minor updates for OpenCode-specific display |

---

## 7. Testing

| Test Type | What to Test |
|-----------|-------------|
| Unit | `OpenCodeProvider::fetch_models()` with mock HTTP |
| Unit | Anonymous vs key mode request building |
| Unit | SSE stream parsing (reuse openai tests) |
| Integration | Startup dialog rendering |
| Integration | Config save/load after key acquisition |
| Manual | OAuth flow end-to-end |
| Manual | Anonymous rate limit behavior |
| Manual | Model switching via fetched model list |

---

## 8. Security Considerations

- API keys stored in `~/.coder/config.toml` (same as other providers)
- No hardcoded keys in source code
- OAuth tokens encrypted in memory during flow
- Rate limiting enforced server-side by OpenCode
- SSL/TLS enforced for all API calls to opencode.ai
