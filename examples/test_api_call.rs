//! 测试 Zazaz API 调用
//!
//! 使用方法：
//! ```bash
//! cargo run --example test_api_call
//! ```

use cadagent::bridge::vlm_client::{VlmClient, VlmConfig};

fn main() {
    // 初始化环境变量（从 .env 文件加载）
    cadagent::init_env();

    println!("=== 测试 Zazaz API 调用 ===\n");

    // 创建 VLM 客户端
    let config = match VlmConfig::default_zazaz() {
        Ok(c) => c,
        Err(e) => {
            println!("❌ 配置加载失败：{}", e);
            return;
        }
    };

    let client = VlmClient::new(config);

    // 准备测试消息
    let messages = &[
        ("system", "你是一个 CAD 几何推理专家。请简洁回答。"),
        ("user", "你好，请简单介绍一下你自己。"),
    ];

    println!("发送请求到 Zazaz API...");
    println!();

    // 调用 API
    match client.chat_completions_blocking(messages) {
        Ok(response) => {
            println!("✅ API 调用成功!\n");
            println!("回答：");
            println!("---");
            for choice in &response.choices {
                println!("{}", choice.message.content);
            }
            println!("---\n");

            // 显示使用统计
            if let Some(usage) = &response.usage {
                println!("Token 使用：");
                println!("  输入：{} tokens", usage.prompt_tokens);
                println!("  输出：{} tokens", usage.completion_tokens);
                println!("  总计：{} tokens", usage.total_tokens);
            }
        }
        Err(e) => {
            println!("❌ API 调用失败：{}", e);
            println!();
            println!("可能的原因：");
            println!("  1. 网络连接问题");
            println!("  2. API Key 无效或过期");
            println!("  3. 服务暂时不可用");
        }
    }

    println!("\n=== 测试完成 ===");
}
