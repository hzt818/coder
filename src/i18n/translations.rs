/// Supported locales
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Locale {
    En,
    Ja,
    ZhHans,
    PtBr,
}

/// Message identifiers for common UI strings
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MessageId {
    StatusReady,
    StatusThinking,
    StatusStreaming,
    HelpNavigation,
    HelpEditing,
    HelpActions,
    PromptSubmit,
    PromptCancel,
    ErrorGeneric,
    ErrorNetwork,
    ErrorPermission,
    ModePlan,
    ModeAgent,
    ModeYolo,
    ReasoningOff,
    ReasoningLow,
    ReasoningHigh,
    ReasoningMax,
    ReasoningAuto,
    ToolRunning,
    ToolSuccess,
    ToolFailed,
}

/// Translate a message ID to the given locale's string.
/// Falls back to English for unknown locales.
pub fn translate(locale: Locale, id: MessageId) -> &'static str {
    match locale {
        Locale::En => translate_en(id),
        Locale::Ja => translate_ja(id),
        Locale::ZhHans => translate_zh_hans(id),
        Locale::PtBr => translate_pt_br(id),
    }
}

fn translate_en(id: MessageId) -> &'static str {
    match id {
        MessageId::StatusReady => "Ready",
        MessageId::StatusThinking => "Thinking...",
        MessageId::StatusStreaming => "Generating...",
        MessageId::HelpNavigation => "Help: Navigation",
        MessageId::HelpEditing => "Help: Editing",
        MessageId::HelpActions => "Help: Actions",
        MessageId::PromptSubmit => "Submit",
        MessageId::PromptCancel => "Cancel",
        MessageId::ErrorGeneric => "An error occurred",
        MessageId::ErrorNetwork => "Network error",
        MessageId::ErrorPermission => "Permission denied",
        MessageId::ModePlan => "Plan",
        MessageId::ModeAgent => "Agent",
        MessageId::ModeYolo => "YOLO",
        MessageId::ReasoningOff => "Off",
        MessageId::ReasoningLow => "Low",
        MessageId::ReasoningHigh => "High",
        MessageId::ReasoningMax => "Max",
        MessageId::ReasoningAuto => "Auto",
        MessageId::ToolRunning => "Running tool...",
        MessageId::ToolSuccess => "Success",
        MessageId::ToolFailed => "Failed",
    }
}

fn translate_ja(id: MessageId) -> &'static str {
    match id {
        MessageId::StatusReady => "準備完了",
        MessageId::StatusThinking => "考え中...",
        MessageId::StatusStreaming => "生成中...",
        MessageId::HelpNavigation => "ヘルプ: ナビゲーション",
        MessageId::HelpEditing => "ヘルプ: 編集",
        MessageId::HelpActions => "ヘルプ: アクション",
        MessageId::PromptSubmit => "送信",
        MessageId::PromptCancel => "キャンセル",
        MessageId::ErrorGeneric => "エラーが発生しました",
        MessageId::ErrorNetwork => "ネットワークエラー",
        MessageId::ErrorPermission => "権限がありません",
        MessageId::ModePlan => "計画",
        MessageId::ModeAgent => "エージェント",
        MessageId::ModeYolo => "YOLO",
        MessageId::ReasoningOff => "オフ",
        MessageId::ReasoningLow => "低",
        MessageId::ReasoningHigh => "高",
        MessageId::ReasoningMax => "最大",
        MessageId::ReasoningAuto => "自動",
        MessageId::ToolRunning => "ツール実行中...",
        MessageId::ToolSuccess => "成功",
        MessageId::ToolFailed => "失敗",
    }
}

fn translate_zh_hans(id: MessageId) -> &'static str {
    match id {
        MessageId::StatusReady => "就绪",
        MessageId::StatusThinking => "思考中...",
        MessageId::StatusStreaming => "生成中...",
        MessageId::HelpNavigation => "帮助: 导航",
        MessageId::HelpEditing => "帮助: 编辑",
        MessageId::HelpActions => "帮助: 操作",
        MessageId::PromptSubmit => "提交",
        MessageId::PromptCancel => "取消",
        MessageId::ErrorGeneric => "发生错误",
        MessageId::ErrorNetwork => "网络错误",
        MessageId::ErrorPermission => "权限拒绝",
        MessageId::ModePlan => "计划",
        MessageId::ModeAgent => "智能体",
        MessageId::ModeYolo => "YOLO",
        MessageId::ReasoningOff => "关闭",
        MessageId::ReasoningLow => "低",
        MessageId::ReasoningHigh => "高",
        MessageId::ReasoningMax => "最大",
        MessageId::ReasoningAuto => "自动",
        MessageId::ToolRunning => "工具运行中...",
        MessageId::ToolSuccess => "成功",
        MessageId::ToolFailed => "失败",
    }
}

fn translate_pt_br(id: MessageId) -> &'static str {
    match id {
        MessageId::StatusReady => "Pronto",
        MessageId::StatusThinking => "Pensando...",
        MessageId::StatusStreaming => "Gerando...",
        MessageId::HelpNavigation => "Ajuda: Navegação",
        MessageId::HelpEditing => "Ajuda: Edição",
        MessageId::HelpActions => "Ajuda: Ações",
        MessageId::PromptSubmit => "Enviar",
        MessageId::PromptCancel => "Cancelar",
        MessageId::ErrorGeneric => "Ocorreu um erro",
        MessageId::ErrorNetwork => "Erro de rede",
        MessageId::ErrorPermission => "Permissão negada",
        MessageId::ModePlan => "Plano",
        MessageId::ModeAgent => "Agente",
        MessageId::ModeYolo => "YOLO",
        MessageId::ReasoningOff => "Desligado",
        MessageId::ReasoningLow => "Baixo",
        MessageId::ReasoningHigh => "Alto",
        MessageId::ReasoningMax => "Máximo",
        MessageId::ReasoningAuto => "Auto",
        MessageId::ToolRunning => "Executando...",
        MessageId::ToolSuccess => "Sucesso",
        MessageId::ToolFailed => "Falhou",
    }
}

/// Return all (MessageId, English string) pairs, useful for testing.
pub fn all_messages() -> Vec<(MessageId, &'static str)> {
    vec![
        (MessageId::StatusReady, "Ready"),
        (MessageId::StatusThinking, "Thinking..."),
        (MessageId::StatusStreaming, "Generating..."),
        (MessageId::HelpNavigation, "Help: Navigation"),
        (MessageId::HelpEditing, "Help: Editing"),
        (MessageId::HelpActions, "Help: Actions"),
        (MessageId::PromptSubmit, "Submit"),
        (MessageId::PromptCancel, "Cancel"),
        (MessageId::ErrorGeneric, "An error occurred"),
        (MessageId::ErrorNetwork, "Network error"),
        (MessageId::ErrorPermission, "Permission denied"),
        (MessageId::ModePlan, "Plan"),
        (MessageId::ModeAgent, "Agent"),
        (MessageId::ModeYolo, "YOLO"),
        (MessageId::ReasoningOff, "Off"),
        (MessageId::ReasoningLow, "Low"),
        (MessageId::ReasoningHigh, "High"),
        (MessageId::ReasoningMax, "Max"),
        (MessageId::ReasoningAuto, "Auto"),
        (MessageId::ToolRunning, "Running tool..."),
        (MessageId::ToolSuccess, "Success"),
        (MessageId::ToolFailed, "Failed"),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::i18n::{current_locale, set_locale, tr, tr_with_locale};

    #[test]
    fn test_all_locales_return_non_empty() {
        let locales = [Locale::En, Locale::Ja, Locale::ZhHans, Locale::PtBr];
        for locale in &locales {
            for (id, _) in all_messages() {
                let result = translate(*locale, id);
                assert!(
                    !result.is_empty(),
                    "Locale {:?} returned empty string for {:?}",
                    locale,
                    id
                );
            }
        }
    }

    #[test]
    fn test_english_translations_are_correct() {
        for (id, expected) in all_messages() {
            assert_eq!(translate(Locale::En, id), expected);
        }
    }

    #[test]
    fn test_japanese_translations() {
        assert_eq!(translate(Locale::Ja, MessageId::StatusReady), "準備完了");
        assert_eq!(
            translate(Locale::Ja, MessageId::StatusThinking),
            "考え中..."
        );
        assert_eq!(
            translate(Locale::Ja, MessageId::ErrorGeneric),
            "エラーが発生しました"
        );
    }

    #[test]
    fn test_chinese_translations() {
        assert_eq!(translate(Locale::ZhHans, MessageId::StatusReady), "就绪");
        assert_eq!(
            translate(Locale::ZhHans, MessageId::StatusThinking),
            "思考中..."
        );
        assert_eq!(
            translate(Locale::ZhHans, MessageId::ErrorGeneric),
            "发生错误"
        );
    }

    #[test]
    fn test_portuguese_translations() {
        assert_eq!(translate(Locale::PtBr, MessageId::StatusReady), "Pronto");
        assert_eq!(
            translate(Locale::PtBr, MessageId::StatusThinking),
            "Pensando..."
        );
        assert_eq!(
            translate(Locale::PtBr, MessageId::ErrorGeneric),
            "Ocorreu um erro"
        );
    }

    #[test]
    fn test_locale_lifecycle() {
        // Test default state: English
        assert_eq!(current_locale(), Locale::En);

        // Set locale to Japanese
        set_locale(Locale::Ja);
        // OnceLock can only be set once, so subsequent calls are no-ops.

        // tr() reads from the (possibly already-set) current locale
        let result = tr(MessageId::StatusReady);
        assert!(!result.is_empty());
    }

    #[test]
    fn test_tr_with_locale_override() {
        assert_eq!(
            tr_with_locale(Locale::PtBr, MessageId::ToolSuccess),
            "Sucesso"
        );
        assert_eq!(tr_with_locale(Locale::Ja, MessageId::ToolSuccess), "成功");
    }
}
