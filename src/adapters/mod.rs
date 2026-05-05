//! IM adapters — connect Coder to instant messaging platforms.
//!
//! Supported platforms:
//! - **Telegram** — via Bot API (long polling)
//! - **Feishu/Lark** — via Feishu Open API
//!
//! Each adapter implements the [`ImAdapter`] trait and runs as an
//! independent message loop, relaying messages to the agent loop.

pub mod telegram;
pub mod feishu;

pub use telegram::{ImAdapter, TelegramAdapter, TelegramConfig, TelegramMessage};
pub use feishu::{FeishuAdapter, FeishuConfig};
