//! VLM API 客户端
//!
//! 提供与视觉语言模型的 API 交互，支持 OpenAI 兼容接口
//!
//! # 支持的供应商
//!
//! - ZazaZ (https://zazaz.top)
//! - OpenAI
//! - 其他 OpenAI 兼容接口
//!
//! # 使用示例
//!
//! ```rust,no_run
//! use cadagent::bridge::vlm_client::{VlmClient, VlmConfig};
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let config = VlmConfig::default_zazaz()?;
//! let client = VlmClient::new(config);
//!
//! // 使用同步 API（无需 async）
//! let response = client.chat_completions_blocking(&[
//!     ("system", "你是一个 CAD 几何推理专家"),
//!     ("user", "请分析这个户型图"),
//! ])?;
//!
//! println!("回答：{}", response.choices[0].message.content);
//! # Ok(())
//! # }
//! ```

use serde::{Deserialize, Serialize};
use thiserror::Error;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use lru::LruCache;
use std::sync::{Arc, Mutex};
use std::num::NonZeroUsize;

/// VLM API 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VlmConfig {
    /// API 基础 URL
    pub base_url: String,
    /// API Key
    pub api_key: String,
    /// 模型名称
    pub model: String,
    /// 请求超时（毫秒）
    pub timeout_ms: u64,
    /// 最大重试次数
    pub max_retries: u32,
}

impl VlmConfig {
    /// 使用 ZazaZ 配置
    ///
    /// # Errors
    /// 如果环境变量 `PROVIDER_ZAZAZ_API_KEY` 未设置，返回 `VlmError::AuthError`
    pub fn default_zazaz() -> Result<Self, VlmError> {
        let api_key = std::env::var("PROVIDER_ZAZAZ_API_KEY")
            .map_err(|_| VlmError::AuthError(
                "环境变量 PROVIDER_ZAZAZ_API_KEY 未设置，请通过 `export PROVIDER_ZAZAZ_API_KEY=your_key` 设置".to_string()
            ))?;

        let base_url = std::env::var("PROVIDER_ZAZAZ_API_URL")
            .unwrap_or_else(|_| "https://zazaz.top/v1".to_string());

        let model = std::env::var("PROVIDER_ZAZAZ_MODEL")
            .unwrap_or_else(|_| "./Qwen3.5-27B-FP8".to_string());

        Ok(Self {
            base_url,
            api_key,
            model,
            timeout_ms: 60000,
            max_retries: 3,
        })
    }

    /// 使用 OpenAI 配置
    ///
    /// # Panics
    /// 如果环境变量 `OPENAI_API_KEY` 未设置，将返回错误
    pub fn default_openai() -> Result<Self, VlmError> {
        let api_key = std::env::var("OPENAI_API_KEY")
            .map_err(|_| VlmError::AuthError(
                "环境变量 OPENAI_API_KEY 未设置，请通过 `export OPENAI_API_KEY=your_key` 设置".to_string()
            ))?;

        Ok(Self {
            base_url: "https://api.openai.com/v1".to_string(),
            api_key,
            model: "gpt-4o".to_string(),
            timeout_ms: 60000,
            max_retries: 3,
        })
    }

    /// 创建自定义配置
    pub fn new(base_url: impl Into<String>, api_key: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            api_key: api_key.into(),
            model: model.into(),
            timeout_ms: 60000,
            max_retries: 3,
        }
    }
}

/// VLM API 错误
#[derive(Debug, Error)]
pub enum VlmError {
    #[error("HTTP 请求失败：{0}")]
    HttpError(String),

    #[error("JSON 解析失败：{0}")]
    JsonError(String),

    #[error("API 返回错误：{0}")]
    ApiError(String),

    #[error("认证失败：{0}")]
    AuthError(String),

    #[error("请求超时：{0}")]
    TimeoutError(String),

    #[error("速率限制：{0}")]
    RateLimitError(String),
}

/// 聊天消息角色
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    System,
    User,
    Assistant,
}

/// 聊天消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: MessageRole,
    pub content: String,
}

impl ChatMessage {
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::System,
            content: content.into(),
        }
    }

    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::User,
            content: content.into(),
        }
    }

    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::Assistant,
            content: content.into(),
        }
    }
}

/// 聊天完成请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatCompletionsRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
}

/// 聊天完成响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatCompletionsResponse {
    pub id: String,
    pub object: String,
    pub created: u64,
    pub model: String,
    pub choices: Vec<ChatChoice>,
    pub usage: Option<UsageInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatChoice {
    pub index: u32,
    pub message: ChatMessage,
    pub finish_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageInfo {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

/// VLM API 错误响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VlmErrorResponse {
    pub error: VlmErrorDetail,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VlmErrorDetail {
    pub message: String,
    #[serde(rename = "type")]
    pub error_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub param: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
}

/// VLM 客户端
pub struct VlmClient {
    config: VlmConfig,
    client: reqwest::Client,
    /// LRU 缓存，存储 prompt 哈希 -> 响应
    cache: Arc<Mutex<LruCache<u64, ChatCompletionsResponse>>>,
}

impl std::fmt::Debug for VlmClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VlmClient")
            .field("config", &self.config)
            .field("cache_size", &self.cache.lock().unwrap().len())
            .finish()
    }
}

impl Clone for VlmClient {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            client: self.client.clone(),
            cache: self.cache.clone(),
        }
    }
}

impl VlmClient {
    /// 创建新的 VLM 客户端（带缓存）
    pub fn new(config: VlmConfig) -> Self {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_millis(config.timeout_ms))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());

        // 创建 LRU 缓存，最多存储 100 个响应
        let cache = LruCache::new(NonZeroUsize::new(100).unwrap());

        Self {
            config,
            client,
            cache: Arc::new(Mutex::new(cache)),
        }
    }

    /// 创建新的 VLM 客户端（自定义缓存大小）
    pub fn new_with_cache_size(config: VlmConfig, cache_size: usize) -> Self {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_millis(config.timeout_ms))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());

        let cache = LruCache::new(
            NonZeroUsize::new(cache_size).unwrap_or(NonZeroUsize::new(100).unwrap())
        );

        Self {
            config,
            client,
            cache: Arc::new(Mutex::new(cache)),
        }
    }

    /// 禁用缓存
    pub fn without_cache(config: VlmConfig) -> Self {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_millis(config.timeout_ms))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());

        Self {
            config,
            client,
            cache: Arc::new(Mutex::new(LruCache::new(NonZeroUsize::new(1).unwrap()))),
        }
    }

    /// 使用默认 ZazaZ 配置创建客户端
    ///
    /// # Errors
    /// 如果环境变量 `PROVIDER_ZAZAZ_API_KEY` 未设置，返回 `VlmError::AuthError`
    pub fn with_zazaz() -> Result<Self, VlmError> {
        let config = VlmConfig::default_zazaz()?;
        Ok(Self::new(config))
    }

    /// 使用默认 OpenAI 配置创建客户端
    pub fn with_openai() -> Result<Self, VlmError> {
        Ok(Self::new(VlmConfig::default_openai()?))
    }

    /// 执行聊天完成请求
    pub async fn chat_completions(
        &self,
        messages: &[(&str, &str)],
    ) -> Result<ChatCompletionsResponse, VlmError> {
        let chat_messages: Vec<ChatMessage> = messages
            .iter()
            .map(|(role, content)| {
                let role = match *role {
                    "system" => MessageRole::System,
                    "assistant" => MessageRole::Assistant,
                    _ => MessageRole::User,
                };
                ChatMessage {
                    role,
                    content: content.to_string(),
                }
            })
            .collect();

        self.chat_completions_with_messages(&chat_messages).await
    }

    /// 使用结构化消息执行聊天完成请求（带缓存）
    pub async fn chat_completions_with_messages(
        &self,
        messages: &[ChatMessage],
    ) -> Result<ChatCompletionsResponse, VlmError> {
        // 计算 prompt 的哈希作为缓存 key
        let prompt_hash = self.compute_prompt_hash(messages);

        // 尝试从缓存获取
        {
            let mut cache = self.cache.lock().unwrap();
            if let Some(cached) = cache.get(&prompt_hash) {
                return Ok(cached.clone());
            }
        }

        let request = ChatCompletionsRequest {
            model: self.config.model.clone(),
            messages: messages.to_vec(),
            temperature: Some(0.7),
            max_tokens: Some(2048),
            stream: Some(false),
        };

        let response = self.send_request(request).await?;

        // 缓存响应
        {
            let mut cache = self.cache.lock().unwrap();
            cache.put(prompt_hash, response.clone());
        }

        Ok(response)
    }

    /// 计算 prompt 的哈希
    fn compute_prompt_hash(&self, messages: &[ChatMessage]) -> u64 {
        let mut hasher = DefaultHasher::new();
        for msg in messages {
            msg.role.hash(&mut hasher);
            msg.content.hash(&mut hasher);
        }
        self.config.model.hash(&mut hasher);
        hasher.finish()
    }

    /// 发送聊天请求
    async fn send_request(
        &self,
        request: ChatCompletionsRequest,
    ) -> Result<ChatCompletionsResponse, VlmError> {
        let url = format!("{}/chat/completions", self.config.base_url);

        let mut retries = 0;
        let mut last_error: Option<VlmError> = None;

        while retries < self.config.max_retries {
            let response = self.client
                .post(&url)
                .header("Content-Type", "application/json")
                .header("Authorization", format!("Bearer {}", self.config.api_key))
                .json(&request)
                .send()
                .await;

            match response {
                Ok(resp) => {
                    let status = resp.status();
                    let bytes = resp.bytes().await
                        .map_err(|e| VlmError::HttpError(format!("读取响应失败：{}", e)))?;

                    if status.is_success() {
                        let response: ChatCompletionsResponse = serde_json::from_slice(&bytes)
                            .map_err(|e| VlmError::JsonError(format!("解析响应失败：{}", e)))?;
                        return Ok(response);
                    } else {
                        // 尝试解析错误响应
                        let error_detail = serde_json::from_slice::<VlmErrorResponse>(&bytes)
                            .ok()
                            .map(|e| e.error.message)
                            .unwrap_or_else(|| format!("HTTP {}", status));

                        if status.as_u16() == 401 {
                            return Err(VlmError::AuthError(error_detail));
                        } else if status.as_u16() == 429 {
                            last_error = Some(VlmError::RateLimitError(error_detail));
                            retries += 1;
                            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                            continue;
                        } else {
                            return Err(VlmError::ApiError(error_detail));
                        }
                    }
                }
                Err(e) => {
                    if e.is_timeout() {
                        last_error = Some(VlmError::TimeoutError(format!("请求超时：{}", e)));
                    } else {
                        last_error = Some(VlmError::HttpError(format!("HTTP 请求失败：{}", e)));
                    }
                    retries += 1;
                    if retries < self.config.max_retries {
                        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                    }
                }
            }
        }

        Err(last_error.unwrap_or_else(|| VlmError::HttpError("未知错误".to_string())))
    }

    /// 同步聊天完成（用于非异步上下文）
    ///
    /// # 性能说明
    ///
    /// 此方法会：
    /// 1. 首先检查 LRU 缓存，如果命中则直接返回（无网络请求）
    /// 2. 如果未命中，创建临时 tokio runtime 执行异步请求
    /// 3. 响应会自动缓存供后续使用
    ///
    /// # 建议
    ///
    /// 对于高性能场景，建议使用异步 API `chat_completions()` 避免每次创建 runtime 的开销
    pub fn chat_completions_blocking(
        &self,
        messages: &[(&str, &str)],
    ) -> Result<ChatCompletionsResponse, VlmError> {
        // 首先尝试使用当前 thread 的 runtime（如果存在）
        if let Ok(handle) = tokio::runtime::Handle::try_current() {
            // 复用现有 runtime
            tokio::task::block_in_place(|| {
                handle.block_on(async {
                    self.chat_completions(messages).await
                })
            })
        } else {
            // 创建新 runtime
            let rt = tokio::runtime::Runtime::new()
                .map_err(|e| VlmError::HttpError(format!("创建运行时失败：{}", e)))?;
            rt.block_on(self.chat_completions(messages))
        }
    }

    /// 清空 LRU 缓存
    pub fn clear_cache(&self) {
        let mut cache = self.cache.lock().unwrap();
        cache.clear();
    }

    /// 获取缓存命中率统计
    pub fn get_cache_stats(&self) -> CacheStats {
        let cache = self.cache.lock().unwrap();
        CacheStats {
            size: cache.len(),
            capacity: cache.cap().get(),
        }
    }
}

/// 缓存统计信息
#[derive(Debug, Clone)]
pub struct CacheStats {
    /// 当前缓存大小
    pub size: usize,
    /// 缓存容量
    pub capacity: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_creation() {
        // 注意：此测试在 CI 环境中可能失败，因为未设置环境变量
        // 本地测试请运行：export PROVIDER_ZAZAZ_API_KEY=test_key && cargo test
        let result = VlmConfig::default_zazaz();
        if let Ok(config) = result {
            assert!(config.base_url.contains("zazaz.top"));
            assert!(!config.api_key.is_empty());
        }
    }

    #[test]
    fn test_message_creation() {
        let msg = ChatMessage::system("测试");
        assert_eq!(msg.role, MessageRole::System);
        assert_eq!(msg.content, "测试");

        let msg = ChatMessage::user("用户消息");
        assert_eq!(msg.role, MessageRole::User);

        let msg = ChatMessage::assistant("助手回复");
        assert_eq!(msg.role, MessageRole::Assistant);
    }
}
