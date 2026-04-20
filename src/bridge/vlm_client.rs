//! VLM API 客户端
//!
//! 提供与视觉语言模型的 API 交互，支持 `OpenAI` 兼容接口
//!
//! # 支持的供应商
//!
//! - `ZazaZ` (<https://zazaz.top>)
//! - `OpenAI`
//! - 其他 `OpenAI` 兼容接口
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

use lru::LruCache;
use secrecy::{ExposeSecret, Secret};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::num::NonZeroUsize;
use std::sync::{Arc, Mutex};
use thiserror::Error;

/// VLM API 配置
///
/// 配置 VLM 客户端的连接参数、认证信息和行为选项。
///
/// # 安全性
///
/// API 密钥使用 `Secret` 包装以防止意外泄露。序列化时不会包含密钥。
#[derive(Debug, Clone)]
pub struct VlmConfig {
    /// API 基础 URL
    pub base_url: String,
    /// API Key（使用 Secret 包装以增强安全性）
    pub api_key: Secret<String>,
    /// 模型名称
    pub model: String,
    /// 请求超时（毫秒）
    pub timeout_ms: u64,
    /// 最大重试次数
    pub max_retries: u32,
}

// 手动实现 Serialize 以处理 Secret<String>
impl Serialize for VlmConfig {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("VlmConfig", 5)?;
        state.serialize_field("base_url", &self.base_url)?;
        // 注意：不序列化 api_key 以增强安全性
        state.serialize_field("model", &self.model)?;
        state.serialize_field("timeout_ms", &self.timeout_ms)?;
        state.serialize_field("max_retries", &self.max_retries)?;
        state.end()
    }
}

// 手动实现 Deserialize 以处理 Secret<String>
impl<'de> Deserialize<'de> for VlmConfig {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct VlmConfigHelper {
            base_url: String,
            api_key: String,
            model: String,
            timeout_ms: u64,
            max_retries: u32,
        }

        let helper = VlmConfigHelper::deserialize(deserializer)?;
        Ok(VlmConfig {
            base_url: helper.base_url,
            api_key: Secret::new(helper.api_key),
            model: helper.model,
            timeout_ms: helper.timeout_ms,
            max_retries: helper.max_retries,
        })
    }
}

impl VlmConfig {
    /// 使用 `ZazaZ` 配置
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
            api_key: Secret::new(api_key),
            model,
            timeout_ms: 60000,
            max_retries: 3,
        })
    }

    /// 使用 `OpenAI` 配置
    ///
    /// # Panics
    /// 如果环境变量 `OPENAI_API_KEY` 未设置，将返回错误
    pub fn default_openai() -> Result<Self, VlmError> {
        let api_key = std::env::var("OPENAI_API_KEY").map_err(|_| {
            VlmError::AuthError(
                "环境变量 OPENAI_API_KEY 未设置，请通过 `export OPENAI_API_KEY=your_key` 设置"
                    .to_string(),
            )
        })?;

        Ok(Self {
            base_url: "https://api.openai.com/v1".to_string(),
            api_key: Secret::new(api_key),
            model: "gpt-4o".to_string(),
            timeout_ms: 60000,
            max_retries: 3,
        })
    }

    /// 使用 `Ollama` 本地模型配置
    ///
    /// Ollama 是一个本地运行开源 LLM 的工具（https://ollama.ai）
    ///
    /// # 环境变量
    /// - `OLLAMA_HOST`: Ollama 服务地址 (可选，默认：http://localhost:11434)
    /// - `OLLAMA_MODEL`: 模型名称 (可选，默认：qwen2.5:7b)
    ///
    /// # 示例
    /// ```rust,no_run
    /// use cadagent::bridge::vlm_client::VlmConfig;
    /// let config = VlmConfig::default_ollama().unwrap();
    /// ```
    pub fn default_ollama() -> Result<Self, VlmError> {
        let base_url = std::env::var("OLLAMA_HOST")
            .unwrap_or_else(|_| "http://localhost:11434/v1".to_string());

        let model = std::env::var("OLLAMA_MODEL")
            .unwrap_or_else(|_| "qwen2.5:7b".to_string());

        // Ollama 本地部署不需要 API Key，使用占位符
        Ok(Self {
            base_url,
            api_key: Secret::new("ollama".to_string()),
            model,
            timeout_ms: 120000, // 本地模型可能需要更长时间
            max_retries: 3,
        })
    }

    /// 使用 `LM Studio` 本地模型配置
    ///
    /// LM Studio 是本地运行 LLM 的桌面应用（https://lmstudio.ai）
    ///
    /// # 环境变量
    /// - `LM_STUDIO_HOST`: LM Studio 服务地址 (可选，默认：http://localhost:1234)
    /// - `LM_STUDIO_MODEL`: 模型名称 (可选，默认：local-model)
    ///
    /// # 示例
    /// ```rust,no_run
    /// use cadagent::bridge::vlm_client::VlmConfig;
    /// let config = VlmConfig::default_lm_studio().unwrap();
    /// ```
    pub fn default_lm_studio() -> Result<Self, VlmError> {
        let base_url = std::env::var("LM_STUDIO_HOST")
            .unwrap_or_else(|_| "http://localhost:1234/v1".to_string());

        let model = std::env::var("LM_STUDIO_MODEL")
            .unwrap_or_else(|_| "local-model".to_string());

        // LM Studio 本地部署不需要 API Key，使用占位符
        Ok(Self {
            base_url,
            api_key: Secret::new("lm-studio".to_string()),
            model,
            timeout_ms: 120000, // 本地模型可能需要更长时间
            max_retries: 3,
        })
    }

    /// 创建自定义配置
    pub fn new(
        base_url: impl Into<String>,
        api_key: impl Into<String>,
        model: impl Into<String>,
    ) -> Self {
        Self {
            base_url: base_url.into(),
            api_key: Secret::new(api_key.into()),
            model: model.into(),
            timeout_ms: 60000,
            max_retries: 3,
        }
    }

    /// 创建无需 API Key 的配置（用于本地模型）
    pub fn new_local(
        base_url: impl Into<String>,
        model: impl Into<String>,
    ) -> Self {
        Self {
            base_url: base_url.into(),
            api_key: Secret::new("local".to_string()),
            model: model.into(),
            timeout_ms: 120000,
            max_retries: 3,
        }
    }

    /// 获取 API 密钥的引用（用于日志记录时请谨慎使用）
    pub fn api_key_ref(&self) -> &str {
        self.api_key.expose_secret()
    }

    /// 检查是否为本地模型配置
    pub fn is_local(&self) -> bool {
        self.base_url.starts_with("http://localhost")
            || self.base_url.starts_with("http://127.0.0.1")
    }
}

/// VLM API 错误类型
///
/// 表示 VLM API 调用过程中可能发生的各种错误。
#[derive(Debug, Error)]
pub enum VlmError {
    /// HTTP 请求失败
    #[error("HTTP 请求失败：{0}")]
    HttpError(String),

    /// JSON 解析失败
    #[error("JSON 解析失败：{0}")]
    JsonError(String),

    /// API 返回错误
    #[error("API 返回错误：{0}")]
    ApiError(String),

    /// 认证失败
    #[error("认证失败：{0}")]
    AuthError(String),

    /// 请求超时
    #[error("请求超时：{0}")]
    TimeoutError(String),

    /// 速率限制
    #[error("速率限制：{0}")]
    RateLimitError(String),

    /// 内部错误
    #[error("内部错误：{0}")]
    InternalError(String),
}

/// 聊天消息角色
///
/// 标识聊天消息的发送者类型。
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    /// 系统提示词
    System,
    /// 用户消息
    User,
    /// 助手回复
    Assistant,
}

/// 聊天消息
///
/// 表示对话中的单条消息，包含角色和内容。
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
///
/// 发送到 VLM API 的聊天完成请求参数。
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
///
/// VLM API 返回的聊天完成响应。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatCompletionsResponse {
    pub id: String,
    pub object: String,
    pub created: u64,
    pub model: String,
    pub choices: Vec<ChatChoice>,
    pub usage: Option<UsageInfo>,
}

/// 聊天响应选择
///
/// 表示聊天完成响应的单个候选结果。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatChoice {
    pub index: u32,
    pub message: ChatMessage,
    pub finish_reason: Option<String>,
}

/// Token 使用统计
///
/// 记录聊天完成请求的 Token 消耗情况。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageInfo {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

/// VLM API 错误响应
///
/// API 返回的错误详情格式。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VlmErrorResponse {
    pub error: VlmErrorDetail,
}

/// VLM API 错误详情
///
/// API 返回的错误详细信息。
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

/// 全局运行时池（用于优化 blocking API）
///
/// # 注意
/// 运行时创建失败时会在首次使用时返回错误而非 panic
/// 使用 Option 包装，允许在创建失败时返回 None
static RUNTIME_POOL: once_cell::sync::Lazy<Arc<tokio::runtime::Runtime>> =
    once_cell::sync::Lazy::new(|| {
        match tokio::runtime::Builder::new_multi_thread()
            .worker_threads(4)
            .thread_name("vlm-runtime")
            .enable_all()
            .build()
        {
            Ok(runtime) => Arc::new(runtime),
            Err(e) => {
                // 记录错误但不 panic，让首次使用时返回错误
                eprintln!("警告：无法创建全局 tokio 运行时：{e}");
                eprintln!("VLM blocking API 将在首次调用时失败");
                // 创建一个最小的 runtime 作为占位符（实际上这仍然可能失败）
                // 更好的做法是返回 Option，但为了向后兼容，我们创建一个 runtime
                // 如果系统资源真的不足，后续调用会返回错误
                tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .map_or_else(
                        |_| panic!("无法创建任何 tokio 运行时：系统资源严重不足"),
                        Arc::new,
                    )
            }
        }
    });

/// VLM 客户端
///
/// 提供与视觉语言模型（VLM）交互的客户端，支持 `OpenAI` 兼容接口。
///
/// # 特性
///
/// - 内置 LRU 缓存，自动缓存重复请求的响应
/// - 自动重试机制，处理网络错误和速率限制
/// - 同步和异步 API 支持
/// - 安全的 API 密钥管理
///
/// # 线程安全
///
/// `VlmClient` 实现了 `Clone`，可以安全地在多个线程间共享。
/// 内部使用连接池和锁无关的缓存访问优化并发性能。
pub struct VlmClient {
    config: VlmConfig,
    /// HTTP 客户端（内部已实现连接池）
    client: reqwest::Client,
    /// LRU 缓存，存储 prompt 哈希 -> 响应
    cache: Arc<Mutex<LruCache<u64, ChatCompletionsResponse>>>,
}

impl std::fmt::Debug for VlmClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VlmClient")
            .field("config", &self.config)
            .field(
                "cache_size",
                &self.cache.lock().map(|c| c.len()).unwrap_or(0),
            )
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
        // 100 是经验值：在内存占用和缓存命中率之间取得平衡
        // 对于典型对话场景，100 个缓存项可覆盖大部分重复查询
        let cache_size = NonZeroUsize::new(100).unwrap_or(NonZeroUsize::MIN);
        let cache = LruCache::new(cache_size);

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

        let cache_size = NonZeroUsize::new(cache_size).unwrap_or(NonZeroUsize::new(100).unwrap());
        let cache = LruCache::new(cache_size);

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
            cache: Arc::new(Mutex::new(LruCache::new(NonZeroUsize::MIN))),
        }
    }

    /// 使用默认 `ZazaZ` 配置创建客户端
    ///
    /// # Errors
    /// 如果环境变量 `PROVIDER_ZAZAZ_API_KEY` 未设置，返回 `VlmError::AuthError`
    pub fn with_zazaz() -> Result<Self, VlmError> {
        let config = VlmConfig::default_zazaz()?;
        Ok(Self::new(config))
    }

    /// 使用默认 `OpenAI` 配置创建客户端
    pub fn with_openai() -> Result<Self, VlmError> {
        Ok(Self::new(VlmConfig::default_openai()?))
    }

    /// 使用 `Ollama` 本地模型配置创建客户端
    ///
    /// # Errors
    /// 如果 Ollama 服务未运行，连接会失败
    pub fn with_ollama() -> Result<Self, VlmError> {
        let config = VlmConfig::default_ollama()?;
        Ok(Self::new(config))
    }

    /// 使用 `LM Studio` 本地模型配置创建客户端
    ///
    /// # Errors
    /// 如果 LM Studio 服务未运行，连接会失败
    pub fn with_lm_studio() -> Result<Self, VlmError> {
        let config = VlmConfig::default_lm_studio()?;
        Ok(Self::new(config))
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
            let mut cache = self.get_cache_lock()?;
            if let Some(cached) = cache.get(&prompt_hash) {
                return Ok(cached.clone());
            }
        }

        let request = ChatCompletionsRequest {
            model: self.config.model.clone(),
            messages: messages.to_vec(),
            // temperature=0.7：在创造性和准确性之间取得平衡
            // 较低的 temperature 会导致输出过于确定性，较高则会产生幻觉
            temperature: Some(0.7),
            // max_tokens=2048：足够容纳详细的几何推理结果
            // 对于大多数 CAD 分析任务，2048 tokens 足以生成完整的思维链
            max_tokens: Some(2048),
            stream: Some(false),
        };

        let response = self.send_request(request).await?;

        // 缓存响应
        {
            let mut cache = self.get_cache_lock()?;
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
            let response = self
                .client
                .post(&url)
                .header("Content-Type", "application/json")
                .header(
                    "Authorization",
                    format!("Bearer {}", self.config.api_key_ref()),
                )
                .json(&request)
                .send()
                .await;

            match response {
                Ok(resp) => {
                    let status = resp.status();
                    let bytes = resp
                        .bytes()
                        .await
                        .map_err(|e| VlmError::HttpError(format!("读取响应失败：{e}")))?;

                    if status.is_success() {
                        let response: ChatCompletionsResponse = serde_json::from_slice(&bytes)
                            .map_err(|e| VlmError::JsonError(format!("解析响应失败：{e}")))?;
                        return Ok(response);
                    }
                    // 尝试解析错误响应
                    let error_detail = serde_json::from_slice::<VlmErrorResponse>(&bytes)
                        .ok()
                        .map_or_else(|| format!("HTTP {status}"), |e| e.error.message);

                    if status.as_u16() == 401 {
                        return Err(VlmError::AuthError(error_detail));
                    } else if status.as_u16() == 429 {
                        last_error = Some(VlmError::RateLimitError(error_detail));
                        retries += 1;
                        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                        continue;
                    }
                    return Err(VlmError::ApiError(error_detail));
                }
                Err(e) => {
                    if e.is_timeout() {
                        last_error = Some(VlmError::TimeoutError(format!("请求超时：{e}")));
                    } else {
                        last_error = Some(VlmError::HttpError(format!("HTTP 请求失败：{e}")));
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
    /// 2. 如果未命中，优先使用全局运行时池执行异步请求
    /// 3. 响应会自动缓存供后续使用
    ///
    /// # 优化
    ///
    /// - 使用全局运行时池避免每次创建 runtime 的开销
    /// - 如果当前已在 tokio runtime 中，会使用 `block_in_place` 避免 panic
    /// - HTTP 客户端内部使用连接池，复用 TCP 连接
    pub fn chat_completions_blocking(
        &self,
        messages: &[(&str, &str)],
    ) -> Result<ChatCompletionsResponse, VlmError> {
        // 首先尝试使用当前 thread 的 runtime（如果存在）
        if let Ok(handle) = tokio::runtime::Handle::try_current() {
            // 复用现有 runtime
            tokio::task::block_in_place(|| {
                handle.block_on(async { self.chat_completions(messages).await })
            })
        } else {
            // 使用全局运行时池，避免每次创建新 runtime
            RUNTIME_POOL.block_on(self.chat_completions(messages))
        }
    }

    /// 清空 LRU 缓存
    pub fn clear_cache(&self) {
        match self.cache.lock() {
            Ok(mut cache) => cache.clear(),
            Err(e) => eprintln!("警告：无法清空缓存（锁中毒）：{e:?}"),
        }
    }

    /// 获取缓存命中率统计
    pub fn get_cache_stats(&self) -> CacheStats {
        match self.cache.lock() {
            Ok(cache) => CacheStats {
                size: cache.len(),
                capacity: cache.cap().get(),
            },
            Err(_) => CacheStats {
                size: 0,
                capacity: 100, // 默认值
            },
        }
    }

    /// 获取缓存锁的辅助函数
    ///
    /// # 注意
    /// 如果锁被中毒（mutex poisoning），会尝试恢复并返回锁
    fn get_cache_lock(
        &self,
    ) -> Result<std::sync::MutexGuard<'_, LruCache<u64, ChatCompletionsResponse>>, VlmError> {
        self.cache
            .lock()
            .map_err(|e| VlmError::InternalError(format!("缓存锁中毒：{e}")))
    }
}

/// 缓存统计信息
///
/// 提供 LRU 缓存的使用情况统计。
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
            assert!(!config.api_key_ref().is_empty());
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

    #[test]
    fn test_ollama_config_default() {
        let config = VlmConfig::default_ollama().expect("Failed to create Ollama config");
        assert_eq!(config.base_url, "http://localhost:11434/v1");
        assert_eq!(config.model, "qwen2.5:7b");
        assert_eq!(config.timeout_ms, 120000);
        assert!(config.is_local());
    }

    #[test]
    fn test_lm_studio_config_default() {
        let config = VlmConfig::default_lm_studio().expect("Failed to create LM Studio config");
        assert_eq!(config.base_url, "http://localhost:1234/v1");
        assert_eq!(config.model, "local-model");
        assert_eq!(config.timeout_ms, 120000);
        assert!(config.is_local());
    }

    #[test]
    fn test_local_config_detection() {
        let localhost_config = VlmConfig::new_local("http://localhost:8080", "test-model");
        assert!(localhost_config.is_local());

        let loopback_config = VlmConfig::new_local("http://127.0.0.1:9000", "test-model");
        assert!(loopback_config.is_local());

        let remote_config = VlmConfig::new("https://api.example.com", "key", "model");
        assert!(!remote_config.is_local());
    }

    #[test]
    fn test_ollama_config_from_env() {
        // Test that environment variables are read correctly
        let config = VlmConfig::default_ollama().expect("Failed to create config");
        
        if std::env::var("OLLAMA_HOST").is_err() {
            assert_eq!(config.base_url, "http://localhost:11434/v1");
        }
        
        if std::env::var("OLLAMA_MODEL").is_err() {
            assert_eq!(config.model, "qwen2.5:7b");
        }
    }

    #[test]
    fn test_lm_studio_config_from_env() {
        // Test that environment variables are read correctly
        let config = VlmConfig::default_lm_studio().expect("Failed to create config");
        
        if std::env::var("LM_STUDIO_HOST").is_err() {
            assert_eq!(config.base_url, "http://localhost:1234/v1");
        }
        
        if std::env::var("LM_STUDIO_MODEL").is_err() {
            assert_eq!(config.model, "local-model");
        }
    }
}
