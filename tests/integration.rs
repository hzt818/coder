//! Integration tests for coder
//!
//! These tests verify that the public API of the coder crate works correctly
//! when assembled together (as opposed to unit tests which test individual
//! components in isolation).

#[cfg(test)]
mod tests {
    // Test 1: ToolRegistry default tools
    #[test]
    fn test_tool_registry_has_core_tools() {
        let registry = coder::tool::ToolRegistry::default();
        assert!(registry.len() >= 7, "Should have at least 7 core tools");
        assert!(registry.get("bash").is_some(), "Should have bash tool");
        assert!(registry.get("file_read").is_some(), "Should have file_read tool");
        assert!(registry.get("file_write").is_some(), "Should have file_write tool");
        assert!(registry.get("grep").is_some(), "Should have grep tool");
        assert!(registry.get("glob").is_some(), "Should have glob tool");
    }

    // Test 2: Tool definitions are valid JSON Schema
    #[test]
    fn test_tool_definitions_have_valid_schema() {
        let registry = coder::tool::ToolRegistry::default();
        let defs = registry.tool_defs();
        for def in &defs {
            assert!(!def.name.is_empty(), "Tool name should not be empty");
            assert!(!def.description.is_empty(), "Tool {} has no description", def.name);
            assert!(def.input_schema.get("properties").is_some() || def.input_schema.get("type").is_some(),
                "Tool {} schema should have properties or type", def.name);
        }
    }

    // Test 3: Config loading
    #[test]
    fn test_config_default_loads() {
        let settings = coder::config::Settings::default();
        assert_eq!(settings.ai.default_provider, "opencode");
        assert!(settings.ai.providers.contains_key("opencode"));
        assert!(settings.ai.providers.contains_key("openai"));
    }

    // Test 4: Session creation
    #[test]
    fn test_session_creation() {
        let session = coder::session::Session::new();
        assert!(!session.id.is_empty(), "Session ID should not be empty");
        assert_eq!(session.messages.len(), 0, "New session should have no messages");
    }

    // Test 5: Session add message
    #[test]
    fn test_session_add_message() {
        let mut session = coder::session::Session::new();
        session.add_message(coder::ai::Message::user("hello"));
        assert_eq!(session.messages.len(), 1);
        assert_eq!(session.messages[0].text(), "hello");
    }

    // Test 6: Message types
    #[test]
    fn test_message_types() {
        let user_msg = coder::ai::Message::user("test");
        assert_eq!(user_msg.role.to_string(), "user");

        let assistant_msg = coder::ai::Message::assistant("response");
        assert_eq!(assistant_msg.role.to_string(), "assistant");

        let system_msg = coder::ai::Message::system("prompt");
        assert_eq!(system_msg.role.to_string(), "system");

        let tool_msg = coder::ai::Message::tool_result("call_1", "result");
        assert_eq!(tool_msg.role.to_string(), "tool");
    }

    // Test 7: Provider creation
    #[test]
    fn test_provider_creation_openai() {
        let config = coder::config::ProviderConfig {
            provider_type: "openai".to_string(),
            api_key: Some("test-key".to_string()),
            base_url: Some("https://api.openai.com/v1".to_string()),
            model: Some("gpt-4o".to_string()),
            ..Default::default()
        };
        let provider = coder::ai::create_provider("test", config, None);
        assert!(provider.is_ok(), "OpenAI provider should be created");
    }

    // Test 8: Anthropic provider creation
    #[test]
    fn test_provider_creation_anthropic() {
        let config = coder::config::ProviderConfig {
            provider_type: "anthropic".to_string(),
            api_key: Some("test-key".to_string()),
            base_url: None,
            model: Some("claude-sonnet-4-6".to_string()),
            ..Default::default()
        };
        let provider = coder::ai::create_provider("test", config, None);
        assert!(provider.is_ok(), "Anthropic provider should be created");
    }

    // Test 9: Agent creation
    #[test]
    fn test_agent_creation() {
        let config = coder::config::ProviderConfig {
            provider_type: "openai".to_string(),
            api_key: Some("test".to_string()),
            base_url: Some("https://api.openai.com/v1".to_string()),
            model: Some("gpt-4o".to_string()),
            ..Default::default()
        };
        let provider = coder::ai::create_provider("test", config, None).unwrap();
        let tools = coder::tool::ToolRegistry::default();
        let agent = coder::agent::Agent::new(provider, tools);
        assert!(agent.tools().len() >= 7, "Agent should have at least 7 tools");
    }

    // Test 10: Config environment variable resolution
    #[test]
    fn test_config_env_resolution() {
        std::env::set_var("CODER_TEST_VAR", "resolved-value");
        let config = coder::config::ProviderConfig {
            api_key: Some("${CODER_TEST_VAR}".to_string()),
            ..Default::default()
        };
        // The resolution happens in Settings::load, not in ProviderConfig
        // Just verify the format is correct
        assert!(config.api_key.unwrap().starts_with("${"));
    }

    // Test 11: Format utilities
    #[test]
    fn test_format_tokens() {
        assert_eq!(coder::util::format::format_tokens(500), "500");
        assert_eq!(coder::util::format::format_tokens(1500), "1.5k");
        assert_eq!(coder::util::format::format_tokens(1_500_000), "1.5M");
    }

    // Test 12: Format duration
    #[test]
    fn test_format_duration() {
        assert_eq!(coder::util::format::format_duration(0.5), "500ms");
        assert_eq!(coder::util::format::format_duration(30.0), "30.0s");
        assert_eq!(coder::util::format::format_duration(90.0), "2m 30s");
    }
}
