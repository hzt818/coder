use clap::Parser;
use std::env;
use std::fs;

#[derive(Parser)]
struct Args {
    #[arg(short, long, default_value = "")]
    message: String,

    /// API key for the provider (overrides env/config)
    #[arg(long)]
    api_key: Option<String>,

    /// Path to TOML config file containing `api_key = "..."`
    #[arg(long)]
    config: Option<String>,
}

fn default_config_path() -> Option<String> {
    env::var("HOME").ok().map(|h| format!("{}/.config/coder/config.toml", h))
}

fn load_api_key(args: &Args) -> Option<String> {
    if let Some(k) = &args.api_key {
        if !k.is_empty() { return Some(k.clone()); }
    }
    if let Ok(k) = env::var("CODER_API_KEY") {
        if !k.is_empty() { return Some(k); }
    }
    let config_path = args.config.clone().or_else(default_config_path);
    if let Some(p) = config_path {
        if let Ok(s) = fs::read_to_string(&p) {
            if let Ok(v) = toml::from_str::<toml::Value>(&s) {
                if let Some(key) = v.get("api_key").and_then(|v| v.as_str()) {
                    return Some(key.to_string());
                }
            }
        }
    }
    None
}

#[tokio::main]
async fn main() {
    let _ = env_logger::try_init();
    let args = Args::parse();
    if args.message.is_empty() {
        println!("coder-cli: hello from scaffold");
    } else {
        println!("Message: {}", args.message);
    }

    match load_api_key(&args) {
        Some(k) => {
            let masked = if k.len() > 8 {
                format!("{}***{}", &k[..4], &k[k.len()-4..])
            } else {
                "****".to_string()
            };
            println!("API key: {}", masked);
        }
        None => println!("API key: not configured (set CODER_API_KEY or --api-key or use config file)"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use tempfile::NamedTempFile;
    use std::io::Write;

    #[test]
    fn api_key_from_arg() {
        let args = Args { message: "".to_string(), api_key: Some("fromarg".to_string()), config: None };
        assert_eq!(load_api_key(&args), Some("fromarg".to_string()));
    }

    #[test]
    fn api_key_from_env() {
        env::remove_var("CODER_API_KEY");
        env::set_var("CODER_API_KEY", "fromenv");
        let args = Args { message: "".to_string(), api_key: None, config: None };
        assert_eq!(load_api_key(&args), Some("fromenv".to_string()));
        env::remove_var("CODER_API_KEY");
    }

    #[test]
    fn api_key_from_config() {
        let mut f = NamedTempFile::new().unwrap();
        writeln!(f, "api_key = \"fromconf\"").unwrap();
        let path = f.path().to_str().unwrap().to_string();
        let args = Args { message: "".to_string(), api_key: None, config: Some(path) };
        assert_eq!(load_api_key(&args), Some("fromconf".to_string()));
    }
}
