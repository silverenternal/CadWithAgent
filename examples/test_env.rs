//! 测试环境变量加载
//!
//! 使用方法：
//! ```bash
//! cargo run --example test_env
//! ```

use cadagent::bridge::vlm_client::{VlmClient, VlmConfig};
use secrecy::ExposeSecret;

fn main() {
    // 初始化环境变量（从 .env 文件加载）
    cadagent::init_env();

    println!("=== 测试 .env 文件加载 ===\n");

    // 1. 直接读取环境变量
    println!("1. 直接读取环境变量:");
    let api_key = std::env::var("PROVIDER_ZAZAZ_API_KEY").unwrap_or_else(|_| "未设置".to_string());
    let api_url = std::env::var("PROVIDER_ZAZAZ_API_URL").unwrap_or_else(|_| "未设置".to_string());
    let model = std::env::var("PROVIDER_ZAZAZ_MODEL").unwrap_or_else(|_| "未设置".to_string());

    println!("   PROVIDER_ZAZAZ_API_KEY = {}", mask_api_key(&api_key));
    println!("   PROVIDER_ZAZAZ_API_URL = {}", api_url);
    println!("   PROVIDER_ZAZAZ_MODEL   = {}", model);
    println!();

    // 2. 使用 VlmConfig::default_zazaz() 创建配置
    println!("2. 使用 VlmConfig::default_zazaz() 创建配置:");
    match VlmConfig::default_zazaz() {
        Ok(config) => {
            println!("   ✅ 配置创建成功!");
            println!("   Base URL: {}", config.base_url);
            println!("   Model:    {}", config.model);
            println!(
                "   API Key:  {}",
                mask_api_key(config.api_key.expose_secret())
            );
        }
        Err(e) => {
            println!("   ❌ 配置创建失败：{}", e);
        }
    }
    println!();

    // 3. 测试 VLM 客户端创建
    println!("3. 测试 VLM 客户端创建:");
    match VlmConfig::default_zazaz() {
        Ok(config) => {
            let client = VlmClient::new(config);
            println!("   ✅ VLM 客户端创建成功!");
            println!("   客户端已准备好调用 API");
            let _ = client; // 避免未使用警告
        }
        Err(e) => {
            println!("   ❌ 无法创建配置：{}", e);
        }
    }
    println!();

    println!("=== 测试完成 ===");
}

/// 隐藏 API Key 的中间部分，仅显示前后缀
fn mask_api_key(key: &str) -> String {
    if key.len() <= 12 {
        return "***".to_string();
    }
    let prefix = &key[..6];
    let suffix = &key[key.len() - 6..];
    format!("{}...{}", prefix, suffix)
}
