//! Finance tool — real-time stock/crypto quotes from Yahoo Finance.
//!
//! Uses Yahoo's public quote endpoint with chart endpoint fallback.
//! Supports symbol normalization for crypto (BTC → BTC-USD).

use async_trait::async_trait;
use super::*;

const USER_AGENT: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15";

pub struct FinanceTool;

#[async_trait]
impl Tool for FinanceTool {
    fn name(&self) -> &str { "finance" }
    fn description(&self) -> &str {
        "Get real-time stock or cryptocurrency quotes from Yahoo Finance. Supports major exchanges and crypto."
    }
    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object", "properties": {
                "symbol": { "type": "string", "description": "Ticker symbol (e.g., AAPL, GOOGL, BTC, ETH)" }
            }, "required": ["symbol"]
        })
    }
    async fn execute(&self, args: serde_json::Value) -> ToolResult {
        let symbol = args.get("symbol").and_then(|s| s.as_str()).unwrap_or("").trim().to_uppercase();
        if symbol.is_empty() { return ToolResult::err("Symbol is required"); }

        // Normalize crypto symbols
        let resolved = if symbol.len() <= 5 && !symbol.contains('-') {
            match symbol.as_str() {
                "BTC" | "ETH" | "SOL" | "DOGE" | "XRP" | "ADA" | "DOT" | "AVAX" => {
                    format!("{}-USD", symbol)
                }
                _ => symbol.clone(),
            }
        } else { symbol.clone() };

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .user_agent(USER_AGENT)
            .build().map_err(|e| format!("Client error: {}", e)).unwrap();

        // Try quote endpoint first
        let quote_url = format!("https://query1.finance.yahoo.com/v7/finance/quote?symbols={}", &resolved);
        match client.get(&quote_url).send().await {
            Ok(resp) if resp.status().is_success() => {
                if let Ok(data) = resp.json::<serde_json::Value>().await {
                    if let Some(quote) = data["quoteResponse"]["result"].as_array().and_then(|r| r.first()) {
                        let price = quote["regularMarketPrice"].as_f64().unwrap_or(0.0);
                        let change = quote["regularMarketChange"].as_f64();
                        let change_pct = quote["regularMarketChangePercent"].as_f64();
                        let name = quote["shortName"].as_str().unwrap_or(&resolved);
                        let currency = quote["currency"].as_str().unwrap_or("USD");
                        let market_state = quote["marketState"].as_str().unwrap_or("REGULAR");

                        let mut result = format!("── {} ({}) ──\n\n", name, resolved);
                        result.push_str(&format!("Price: {:.2} {}\n", price, currency));
                        if let Some(c) = change {
                            result.push_str(&format!("Change: {:.2} ({:.2}%)\n", c, change_pct.unwrap_or(0.0)));
                        }
                        result.push_str(&format!("Market: {}\n", market_state));
                        return ToolResult::ok(result);
                    }
                }
            }
            Ok(resp) if resp.status().as_u16() == 404 || resp.status().as_u16() == 429 => {
                // Fall through to chart fallback
            }
            Ok(_) | Err(_) => {}
        }

        // Fallback: try chart endpoint
        let chart_url = format!("https://query1.finance.yahoo.com/v8/finance/chart/{}?interval=1d&range=5d", &resolved);
        match client.get(&chart_url).send().await {
            Ok(resp) if resp.status().is_success() => {
                if let Ok(data) = resp.json::<serde_json::Value>().await {
                    if let Some(meta) = data["chart"]["result"].as_array().and_then(|r| r.first()).and_then(|r| r.get("meta")) {
                        let price = meta["regularMarketPrice"].as_f64().unwrap_or(0.0);
                        let prev_close = meta["previousClose"].as_f64().unwrap_or(0.0);
                        let name = data["chart"]["result"][0]["meta"]["symbol"].as_str().unwrap_or(&resolved);
                        return ToolResult::ok(format!(
                            "── {} ──\n\nPrice: {:.2}\nPrevious close: {:.2}\n(Chart fallback)\n",
                            name, price, prev_close
                        ));
                    }
                }
            }
            Ok(_) | Err(_) => {}
        }

        ToolResult::err(format!("Could not fetch quote for '{}'. Verify the symbol.", symbol))
    }
    fn requires_permission(&self) -> bool { false }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_name() { assert_eq!(FinanceTool.name(), "finance"); }
    #[tokio::test] async fn test_empty_symbol() { assert!(!FinanceTool.execute(serde_json::json!({})).await.success); }
    #[tokio::test] async fn test_invalid_symbol() {
        let r = FinanceTool.execute(serde_json::json!({"symbol": "ZZZZZZZZ"})).await;
        // Should handle gracefully (error, not panic)
        assert!(!r.success || r.output.len() > 0);
    }
}
