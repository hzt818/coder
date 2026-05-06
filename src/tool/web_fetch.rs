//! Web fetch tool - fetches web page content

use super::*;
use async_trait::async_trait;
use std::net::{IpAddr, Ipv6Addr};

pub struct WebFetchTool;

#[async_trait]
impl Tool for WebFetchTool {
    fn name(&self) -> &str {
        "web_fetch"
    }

    fn description(&self) -> &str {
        "Fetch and read the content of a web page."
    }

    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "url": {
                    "type": "string",
                    "description": "URL to fetch"
                }
            },
            "required": ["url"]
        })
    }

    async fn execute(&self, args: serde_json::Value) -> ToolResult {
        let url = args.get("url").and_then(|u| u.as_str()).unwrap_or("");

        if url.is_empty() {
            return ToolResult::err("URL is required");
        }

        // Security: only allow http/https schemes
        let parsed = match url::Url::parse(url) {
            Ok(u) => u,
            Err(e) => return ToolResult::err(format!("Invalid URL '{}': {}", url, e)),
        };
        if parsed.scheme() != "http" && parsed.scheme() != "https" {
            return ToolResult::err(format!(
                "URL scheme '{}' is not allowed. Only http:// and https:// are supported.",
                parsed.scheme()
            ));
        }

        // SSRF protection: block requests to private/internal IPs
        if let Err(e) = check_ssrf(&parsed).await {
            return ToolResult::err(e);
        }

        match fetch_url(url).await {
            Ok(content) => ToolResult::ok(content),
            Err(e) => ToolResult::err(format!("Failed to fetch URL: {}", e)),
        }
    }
}

async fn fetch_url(url: &str) -> Result<String, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .user_agent("Coder/1.0")
        .build()
        .map_err(|e| format!("HTTP client error: {}", e))?;

    let response = client
        .get(url)
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("HTTP {}", response.status()));
    }

    let content_type = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();

    let body = response
        .text()
        .await
        .map_err(|e| format!("Failed to read response body: {}", e))?;

    // Truncate if too large
    const MAX_SIZE: usize = 500_000;
    let mut result = String::new();
    result.push_str(&format!("URL: {}\n", url));
    result.push_str(&format!("Content-Type: {}\n", content_type));
    result.push_str(&format!("Size: {} bytes\n", body.len()));
    result.push('\n');

    if body.len() > MAX_SIZE {
        let safe_end = body.floor_char_boundary(MAX_SIZE);
        result.push_str(&body[..safe_end]);
        result.push_str(&format!("\n... (truncated, {} bytes total)", body.len()));
    } else {
        result.push_str(&body);
    }

    Ok(result)
}

/// SSRF protection: reject requests to private/internal IP ranges.
async fn check_ssrf(parsed: &url::Url) -> Result<(), String> {
    let host = parsed.host_str().unwrap_or("");
    if host.is_empty() {
        return Err("URL has no host".to_string());
    }

    // Block obvious private hostnames
    let lower = host.to_lowercase();
    if lower == "localhost"
        || lower == "127.0.0.1"
        || lower == "::1"
        || lower.ends_with(".local")
        || lower.ends_with(".internal")
    {
        return Err(format!(
            "SSRF blocked: '{}' is a private/internal hostname",
            host
        ));
    }

    // Resolve hostname and check IP ranges
    if let Ok(addrs) = tokio::net::lookup_host((host, 0)).await {
        for addr in addrs {
            if is_private_ip(addr.ip()) {
                return Err(format!(
                    "SSRF blocked: '{}' resolves to private IP {}",
                    host,
                    addr.ip()
                ));
            }
        }
    }
    // If DNS resolution fails, let the request proceed — it will fail anyway

    Ok(())
}

/// Check if an IP address belongs to a private/internal range
fn is_private_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => {
            // 127.0.0.0/8 (loopback)
            v4.is_loopback() ||
            // 10.0.0.0/8 (private class A)
            v4.octets()[0] == 10 ||
            // 172.16.0.0/12 (private class B)
            (v4.octets()[0] == 172 && (v4.octets()[1] & 0xF0) == 16) ||
            // 192.168.0.0/16 (private class C)
            (v4.octets()[0] == 192 && v4.octets()[1] == 168) ||
            // 169.254.0.0/16 (link-local)
            (v4.octets()[0] == 169 && v4.octets()[1] == 254) ||
            // 100.64.0.0/10 (Carrier-grade NAT)
            v4.octets()[0] == 100 && (v4.octets()[1] & 0xC0) == 64
        }
        IpAddr::V6(v6) => {
            // ::1 (loopback)
            v6 == Ipv6Addr::LOCALHOST ||
            // fe80::/10 (link-local)
            v6.octets()[0] == 0xFE && (v6.octets()[1] & 0xC0) == 0x80
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_web_fetch_tool_name() {
        let tool = WebFetchTool;
        assert_eq!(tool.name(), "web_fetch");
    }

    #[tokio::test]
    async fn test_web_fetch_no_url() {
        let tool = WebFetchTool;
        let result = tool.execute(serde_json::json!({"url": ""})).await;
        assert!(!result.success);
    }
}
