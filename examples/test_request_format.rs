//! 测试请求格式是否正确
//!
//! 使用方法：
//! ```bash
//! cargo run --example test_request_format
//! ```

use cadagent::bridge::vlm_client::{VlmClient, VlmConfig};
use secrecy::ExposeSecret;

fn main() {
    // 初始化环境变量（从 .env 文件加载）
    cadagent::init_env();

    println!("=== 验证请求格式 ===\n");

    // 1. 检查环境变量
    let api_key = std::env::var("PROVIDER_ZAZAZ_API_KEY").expect("API Key 未设置");
    let base_url = std::env::var("PROVIDER_ZAZAZ_API_URL")
        .unwrap_or_else(|_| "https://zazaz.top/v1".to_string());
    let model =
        std::env::var("PROVIDER_ZAZAZ_MODEL").unwrap_or_else(|_| "./Qwen3.5-27B-FP8".to_string());

    println!("1. 环境变量:");
    println!(
        "   API Key: {} (长度：{})",
        mask_api_key(&api_key),
        api_key.len()
    );
    println!("   Base URL: {}", base_url);
    println!("   Model: {}", model);
    println!();

    // 2. 检查 VlmConfig
    let config = VlmConfig::default_zazaz().expect("配置创建失败");
    println!("2. VlmConfig:");
    println!("   base_url: {}", config.base_url);
    println!("   model: {}", config.model);
    println!(
        "   api_key: {} (长度：{})",
        mask_api_key(config.api_key.expose_secret()),
        config.api_key.expose_secret().len()
    );
    println!("   timeout_ms: {}", config.timeout_ms);
    println!("   max_retries: {}", config.max_retries);
    println!();

    // 3. 检查 VlmClient
    let _client = VlmClient::new(config);
    println!("3. VlmClient 创建成功");
    println!();

    // 4. 生成等效的 curl 命令
    println!("4. 等效 curl 命令:");
    let curl_cmd = format!(
        r#"curl {}/v1/chat/completions \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer {}" \
  -d '{{"model":"{}","messages":[{{"role":"user","content":"你好"}}]}}'"#,
        base_url.trim_end_matches("/v1"),
        api_key,
        model
    );
    println!("{}", curl_cmd);
    println!();

    // 5. 检查最终 URL
    println!("5. 最终请求 URL:");
    println!("   {}/chat/completions", base_url);
    println!();

    println!("=== 验证完成 ===");
    println!();
    println!("注意：如果 API 返回 '无法连接到上游 vLLM 服务'，");
    println!("说明是 Zazaz 服务端的问题，不是代码问题。");
}

fn mask_api_key(key: &str) -> String {
    if key.len() <= 12 {
        return "***".to_string();
    }
    let prefix = &key[..6];
    let suffix = &key[key.len() - 6..];
    format!("{}...{}", prefix, suffix)
}
