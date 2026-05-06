//! Simple template engine for custom provider request/response templates

/// Render a template with variable substitution.
/// Supports {{variable}} syntax for simple substitution.
pub fn render_template(template: &str, vars: &std::collections::HashMap<&str, &str>) -> String {
    let mut result = template.to_string();
    for (key, value) in vars {
        let placeholder = format!("{{{{{}}}}}", key);
        result = result.replace(&placeholder, value);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_template() {
        let mut vars = std::collections::HashMap::new();
        vars.insert("name", "world");
        assert_eq!(render_template("Hello, {{name}}!", &vars), "Hello, world!");
    }

    #[test]
    fn test_render_template_empty_vars() {
        let vars = std::collections::HashMap::new();
        assert_eq!(
            render_template("Hello, {{name}}!", &vars),
            "Hello, {{name}}!"
        );
    }
}
