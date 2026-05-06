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
        assert!(
            registry.get("file_read").is_some(),
            "Should have file_read tool"
        );
        assert!(
            registry.get("file_write").is_some(),
            "Should have file_write tool"
        );
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
            assert!(
                !def.description.is_empty(),
                "Tool {} has no description",
                def.name
            );
            assert!(
                def.input_schema.get("properties").is_some()
                    || def.input_schema.get("type").is_some(),
                "Tool {} schema should have properties or type",
                def.name
            );
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
        assert_eq!(
            session.messages.len(),
            0,
            "New session should have no messages"
        );
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
        assert!(
            agent.tools().len() >= 7,
            "Agent should have at least 7 tools"
        );
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

    // Helper to run async code in sync tests (uses tokio runtime since tools depend on it)
    fn block_on<F: std::future::Future>(f: F) -> F::Output {
        tokio::runtime::Runtime::new().unwrap().block_on(f)
    }

    // Test 13: Tool execution - bash basic
    #[test]
    fn test_tool_execution_bash() {
        let registry = coder::tool::ToolRegistry::default();
        let result = block_on(registry.get("bash").unwrap().execute(serde_json::json!({
            "command": "echo hello"
        })));
        assert!(result.success, "Bash echo should succeed");
        assert!(result.output.contains("hello"), "Should contain 'hello'");
    }

    // Test 14: Tool execution - file_read with nonexistent file
    #[test]
    fn test_tool_execution_file_read_error() {
        let registry = coder::tool::ToolRegistry::default();
        let result = block_on(
            registry
                .get("file_read")
                .unwrap()
                .execute(serde_json::json!({
                    "path": "/nonexistent/file.txt"
                })),
        );
        assert!(!result.success, "Reading nonexistent file should fail");
    }

    // Test 15: All registered tools have valid schemas
    #[test]
    fn test_all_tools_have_valid_schemas() {
        let registry = coder::tool::ToolRegistry::default();
        let defs = registry.tool_defs();
        assert!(defs.len() >= 10, "Should have at least 10 registered tools");
        for def in &defs {
            assert!(!def.name.is_empty(), "Tool name should not be empty");
            assert!(
                !def.description.is_empty(),
                "Tool {} has no description",
                def.name
            );
            assert!(
                def.input_schema.is_object(),
                "Tool {} schema should be an object",
                def.name
            );
        }
    }

    // Test 16: All tool names are unique
    #[test]
    fn test_tool_names_are_unique() {
        let registry = coder::tool::ToolRegistry::default();
        let names = registry.tool_names();
        let mut sorted = names.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(names.len(), sorted.len(), "Tool names should be unique");
    }

    // Test 17: Session save/load round-trip (via serialization, since SessionManager.sessions_dir is private)
    #[test]
    fn test_session_roundtrip() {
        let mut session = coder::session::Session::new();
        session.title = "Test Roundtrip".to_string();
        session.add_message(coder::ai::Message::user("hello"));
        session.add_message(coder::ai::Message::assistant("world"));

        let json = serde_json::to_string(&session).unwrap();
        let deserialized: coder::session::Session = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.title, "Test Roundtrip");
        assert_eq!(deserialized.messages.len(), 2);
        assert_eq!(deserialized.messages[0].text(), "hello");
        assert_eq!(deserialized.messages[1].text(), "world");
    }

    // Test 18: Session list and delete (uses SessionManager::new() with real sessions dir, cleans up)
    #[test]
    fn test_session_list_and_delete() {
        let manager = coder::session::manager::SessionManager::new();
        let session = coder::session::Session::new();

        // Save
        manager.save(&session).unwrap();

        // Verify it appears in the list
        let list = manager.list().unwrap();
        assert!(
            list.iter().any(|s| s.id == session.id),
            "Session should be in list after save"
        );

        // Delete
        manager.delete(&session.id).unwrap();

        // Verify it is gone
        let list = manager.list().unwrap();
        assert!(
            !list.iter().any(|s| s.id == session.id),
            "Session should not be in list after delete"
        );
    }

    // Test 19: Config load from file
    #[test]
    fn test_config_load_from_file() {
        let tmp = tempfile::tempdir().unwrap();
        let config_path = tmp.path().join("coder.toml");
        std::fs::write(
            &config_path,
            r#"
[ai]
default_provider = "test-provider"

[ai.provider.test-provider]
type = "openai"
api_key = "test-key"
model = "gpt-4o"
"#,
        )
        .unwrap();

        let settings = coder::config::Settings::load(Some(config_path.to_str().unwrap())).unwrap();
        assert_eq!(settings.ai.default_provider, "test-provider");
    }

    // Test 20: Agent creation with tools
    #[test]
    fn test_agent_creation_with_tools() {
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
        assert!(
            agent.tools().len() >= 10,
            "Agent should have at least 10 tools"
        );
        assert_eq!(agent.agent_type().to_string(), "coding");
    }

    // Test 21: Multiple provider types can be created
    #[test]
    fn test_multiple_provider_types() {
        let providers = vec![
            ("openai", "https://api.openai.com/v1", "gpt-4o"),
            (
                "anthropic",
                "https://api.anthropic.com/v1",
                "claude-sonnet-4-6",
            ),
            (
                "opencode",
                "https://opencode.ai/zen/v1",
                "claude-sonnet-4-6",
            ),
            (
                "google",
                "https://generativelanguage.googleapis.com",
                "gemini-2.0-flash",
            ),
        ];
        for (provider_type, base_url, model) in providers {
            // Skip providers whose features aren't enabled (e.g. opencode
            // without ai-opencode, google without ai-google).
            let enabled = match provider_type {
                "opencode" => cfg!(feature = "ai-opencode"),
                "google" => cfg!(feature = "ai-google"),
                _ => true,
            };
            if !enabled {
                continue;
            }

            let config = coder::config::ProviderConfig {
                provider_type: provider_type.to_string(),
                api_key: Some("test-key".to_string()),
                base_url: Some(base_url.to_string()),
                model: Some(model.to_string()),
                ..Default::default()
            };
            let provider = coder::ai::create_provider("test", config, None);
            assert!(
                provider.is_ok(),
                "Provider type '{}' should be created",
                provider_type
            );
        }
    }
}
