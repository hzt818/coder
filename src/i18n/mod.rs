pub mod translations;

use translations::{Locale, MessageId};
use std::sync::OnceLock;

static CURRENT_LOCALE: OnceLock<Locale> = OnceLock::new();

pub fn set_locale(locale: Locale) {
    let _ = CURRENT_LOCALE.set(locale);
}

pub fn current_locale() -> Locale {
    CURRENT_LOCALE.get().copied().unwrap_or(Locale::En)
}

/// Translate a message ID to the current locale
pub fn tr(id: MessageId) -> &'static str {
    let locale = current_locale();
    translations::translate(locale, id)
}

/// Translate with locale override
pub fn tr_with_locale(locale: Locale, id: MessageId) -> &'static str {
    translations::translate(locale, id)
}
