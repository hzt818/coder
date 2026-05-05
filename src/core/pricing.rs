//! Pricing models and cost estimation
//!
//! Provides per-model cost tables and live token/cost tracking.
//! Supports cache hit/miss cost breakdown for prefix caching.

use std::collections::HashMap;

/// Cost entry for a model
#[derive(Debug, Clone)]
pub struct ModelCost {
    pub input_per_1m: f64,
    pub cache_hit_per_1m: f64,
    pub output_per_1m: f64,
}

/// Aggregated cost estimate for a turn or session
#[derive(Debug, Clone, Default)]
pub struct CostEstimate {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_hit_tokens: u64,
    pub cache_miss_tokens: u64,
    pub input_cost: f64,
    pub output_cost: f64,
    pub total_cost: f64,
}

impl CostEstimate {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add usage from another estimate
    pub fn add(&mut self, other: &CostEstimate) {
        self.input_tokens += other.input_tokens;
        self.output_tokens += other.output_tokens;
        self.cache_hit_tokens += other.cache_hit_tokens;
        self.cache_miss_tokens += other.cache_miss_tokens;
        self.input_cost += other.input_cost;
        self.output_cost += other.output_cost;
        self.total_cost += other.total_cost;
    }

    /// Format total cost as a string (e.g., "$0.0012")
    pub fn formatted_cost(&self) -> String {
        format!("${:.6}", self.total_cost)
    }

    /// Format cost per 1K tokens for display
    pub fn cost_per_1k_input(&self) -> String {
        if self.input_tokens == 0 {
            return "$0.00".to_string();
        }
        let per_1k = self.input_cost / (self.input_tokens as f64 / 1000.0);
        format!("${:.4}", per_1k)
    }
}

/// Pricing table for known models
fn default_pricing() -> HashMap<&'static str, ModelCost> {
    let mut m = HashMap::new();

    // DeepSeek models
    m.insert(
        "deepseek-v4-pro",
        ModelCost {
            input_per_1m: 0.435,
            cache_hit_per_1m: 0.003625,
            output_per_1m: 0.87,
        },
    );
    m.insert(
        "deepseek-v4-flash",
        ModelCost {
            input_per_1m: 0.14,
            cache_hit_per_1m: 0.0028,
            output_per_1m: 0.28,
        },
    );
    m.insert(
        "deepseek-chat",
        ModelCost {
            input_per_1m: 0.14,
            cache_hit_per_1m: 0.0028,
            output_per_1m: 0.28,
        },
    );
    m.insert(
        "deepseek-reasoner",
        ModelCost {
            input_per_1m: 0.14,
            cache_hit_per_1m: 0.0028,
            output_per_1m: 0.28,
        },
    );

    // OpenAI models
    m.insert(
        "gpt-4o",
        ModelCost {
            input_per_1m: 2.50,
            cache_hit_per_1m: 1.25,
            output_per_1m: 10.00,
        },
    );
    m.insert(
        "gpt-4o-mini",
        ModelCost {
            input_per_1m: 0.15,
            cache_hit_per_1m: 0.075,
            output_per_1m: 0.60,
        },
    );
    m.insert(
        "o1",
        ModelCost {
            input_per_1m: 15.00,
            cache_hit_per_1m: 7.50,
            output_per_1m: 60.00,
        },
    );
    m.insert(
        "o3-mini",
        ModelCost {
            input_per_1m: 1.10,
            cache_hit_per_1m: 0.55,
            output_per_1m: 4.40,
        },
    );

    // Anthropic models
    m.insert(
        "claude-sonnet-4-6",
        ModelCost {
            input_per_1m: 3.00,
            cache_hit_per_1m: 0.30,
            output_per_1m: 15.00,
        },
    );
    m.insert(
        "claude-sonnet-4-5",
        ModelCost {
            input_per_1m: 3.00,
            cache_hit_per_1m: 0.30,
            output_per_1m: 15.00,
        },
    );
    m.insert(
        "claude-opus-4-6",
        ModelCost {
            input_per_1m: 15.00,
            cache_hit_per_1m: 1.50,
            output_per_1m: 75.00,
        },
    );
    m.insert(
        "claude-haiku-4-5",
        ModelCost {
            input_per_1m: 0.80,
            cache_hit_per_1m: 0.08,
            output_per_1m: 4.00,
        },
    );

    // Default fallback
    m.insert(
        "default",
        ModelCost {
            input_per_1m: 1.00,
            cache_hit_per_1m: 0.10,
            output_per_1m: 2.00,
        },
    );

    m
}

/// Look up the cost table for a given model name
pub fn model_cost(model: &str) -> ModelCost {
    let pricing = default_pricing();
    // Try exact match first
    if let Some(cost) = pricing.get(model) {
        return cost.clone();
    }
    // Try prefix match
    for (key, cost) in &pricing {
        if model.starts_with(key) || model.contains(key) {
            return cost.clone();
        }
    }
    // Fallback to default
    pricing.get("default").cloned().unwrap_or(ModelCost {
        input_per_1m: 1.0,
        cache_hit_per_1m: 0.1,
        output_per_1m: 2.0,
    })
}

/// Calculate cost estimate from token usage and model name
pub fn calculate_cost(
    model: &str,
    input_tokens: u64,
    output_tokens: u64,
    cache_hit_tokens: u64,
    cache_miss_tokens: u64,
) -> CostEstimate {
    let cost = model_cost(model);

    let input_cost = (cache_hit_tokens as f64 / 1_000_000.0) * cost.cache_hit_per_1m
        + (cache_miss_tokens as f64 / 1_000_000.0) * cost.input_per_1m;

    let output_cost = (output_tokens as f64 / 1_000_000.0) * cost.output_per_1m;

    let total_cost = input_cost + output_cost;

    CostEstimate {
        input_tokens,
        output_tokens,
        cache_hit_tokens,
        cache_miss_tokens,
        input_cost,
        output_cost,
        total_cost,
    }
}

/// Estimate tokens from text length (rough approximation)
pub fn estimate_tokens(text: &str) -> usize {
    // Rough estimate: ~4 chars per token for English text
    // This is a simple heuristic; actual tokenization varies by model
    let char_count = text.chars().count();
    let word_count = text.split_whitespace().count();

    // Weighted: average of chars/4 and words*1.3
    let from_chars = char_count / 4;
    let from_words = (word_count as f64 * 1.3) as usize;

    std::cmp::max(from_chars, from_words).max(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_model_cost_lookup() {
        let cost = model_cost("gpt-4o");
        assert!((cost.input_per_1m - 2.50).abs() < 0.01);

        let cost = model_cost("claude-sonnet-4-6");
        assert!((cost.input_per_1m - 3.00).abs() < 0.01);

        let cost = model_cost("deepseek-v4-flash");
        assert!((cost.input_per_1m - 0.14).abs() < 0.01);
    }

    #[test]
    fn test_model_cost_fallback() {
        let cost = model_cost("unknown-model-xyz");
        assert!((cost.input_per_1m - 1.00).abs() < 0.01);
    }

    #[test]
    fn test_calculate_cost() {
        let result = calculate_cost("gpt-4o", 1000, 500, 500, 500);
        assert_eq!(result.input_tokens, 1000);
        assert_eq!(result.output_tokens, 500);
        assert!(result.total_cost > 0.0);
    }

    #[test]
    fn test_cost_estimate_add() {
        let mut total = CostEstimate::new();
        let a = calculate_cost("gpt-4o", 1000, 500, 500, 500);
        let b = calculate_cost("gpt-4o", 2000, 1000, 1000, 1000);
        total.add(&a);
        total.add(&b);
        assert_eq!(total.input_tokens, 3000);
        assert_eq!(total.output_tokens, 1500);
    }

    #[test]
    fn test_cost_estimate_empty() {
        let est = CostEstimate::new();
        assert_eq!(est.total_cost, 0.0);
        assert_eq!(est.formatted_cost(), "$0.000000");
    }

    #[test]
    fn test_estimate_tokens() {
        let tokens = estimate_tokens("Hello world, this is a test message");
        assert!(tokens >= 1);
        // Short text
        let tokens = estimate_tokens("hi");
        assert!(tokens >= 1);
        // Empty text
        let tokens = estimate_tokens("");
        assert_eq!(tokens, 1);
    }

    #[test]
    fn test_deepseek_pricing() {
        let cost = model_cost("deepseek-v4-pro");
        assert!((cost.input_per_1m - 0.435).abs() < 0.001);
        assert!((cost.cache_hit_per_1m - 0.003625).abs() < 0.0001);
        assert!((cost.output_per_1m - 0.87).abs() < 0.001);
    }
}
