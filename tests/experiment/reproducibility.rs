//! 可重复性包
//!
//! 提供实验可重复性所需的工具，包括随机种子管理、环境捕获、配置导出等。
//!
//! # 使用示例
//!
//! ```rust
//! use experiment::reproducibility::{EnvironmentInfo, SeedManager, ReproducibilityConfig};
//!
//! // 捕获当前环境
//! let env = EnvironmentInfo::capture();
//! env.save_to("environment.json")?;
//!
//! // 设置可重复的随机种子
//! let mut seed_manager = SeedManager::new(42);
//! let seed1 = seed_manager.next_seed();
//! let seed2 = seed_manager.next_seed();
//!
//! // 创建可重复性配置
//! let config = ReproducibilityConfig::new()
//!     .with_seed(42)
//!     .with_deterministic(true)
//!     .with_environment(env);
//!
//! config.save_to("reproducibility_config.json")?;
//! ```

#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// 环境信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvironmentInfo {
    /// 时间戳
    pub timestamp: String,
    /// Rust 版本
    pub rust_version: String,
    /// Cargo 版本
    pub cargo_version: String,
    /// 目标平台
    pub target: String,
    /// 主机平台
    pub host: String,
    /// CPU 核心数
    pub num_cpus: usize,
    /// 内存总量 (MB)
    pub total_memory_mb: Option<u64>,
    /// 操作系统
    pub os: String,
    /// 环境变量
    pub env_vars: HashMap<String, String>,
    /// Git 信息
    pub git_info: Option<GitInfo>,
    /// 依赖版本
    pub dependencies: HashMap<String, String>,
}

/// Git 信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitInfo {
    /// Git commit hash
    pub commit: String,
    /// Git branch
    pub branch: String,
    /// 是否 dirty (有未提交更改)
    pub dirty: bool,
    /// commit 时间
    pub commit_date: String,
    /// commit 消息
    pub commit_message: Option<String>,
}

/// 种子管理器
pub struct SeedManager {
    base_seed: u64,
    counter: u64,
}

impl SeedManager {
    pub fn new(base_seed: u64) -> Self {
        Self {
            base_seed,
            counter: 0,
        }
    }

    /// 生成下一个种子
    pub fn next_seed(&mut self) -> u64 {
        let seed = self.base_seed.wrapping_add(self.counter);
        self.counter += 1;
        seed
    }

    /// 重置计数器
    pub fn reset(&mut self) {
        self.counter = 0;
    }

    /// 设置基础种子
    pub fn set_base_seed(&mut self, seed: u64) {
        self.base_seed = seed;
        self.counter = 0;
    }

    /// 获取基础种子
    pub fn base_seed(&self) -> u64 {
        self.base_seed
    }
}

/// 可重复性配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReproducibilityConfig {
    /// 随机种子
    pub seed: u64,
    /// 是否确定性模式
    pub deterministic: bool,
    /// 环境信息
    pub environment: EnvironmentInfo,
    /// 实验参数
    pub experiment_params: HashMap<String, String>,
    /// 重现说明
    pub reproduction_notes: String,
}

impl ReproducibilityConfig {
    pub fn new() -> Self {
        Self {
            seed: 42,
            deterministic: true,
            environment: EnvironmentInfo::default(),
            experiment_params: HashMap::new(),
            reproduction_notes: String::new(),
        }
    }

    pub fn with_seed(mut self, seed: u64) -> Self {
        self.seed = seed;
        self
    }

    pub fn with_deterministic(mut self, deterministic: bool) -> Self {
        self.deterministic = deterministic;
        self
    }

    pub fn with_environment(mut self, env: EnvironmentInfo) -> Self {
        self.environment = env;
        self
    }

    pub fn with_param(mut self, key: &str, value: &str) -> Self {
        self.experiment_params
            .insert(key.to_string(), value.to_string());
        self
    }

    pub fn with_notes(mut self, notes: &str) -> Self {
        self.reproduction_notes = notes.to_string();
        self
    }

    /// 保存配置
    pub fn save_to(&self, path: &Path) -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let content = serde_json::to_string_pretty(self)?;
        fs::write(path, content)
    }

    /// 从文件加载配置
    pub fn load_from(path: &Path) -> std::io::Result<Self> {
        let content = fs::read_to_string(path)?;
        serde_json::from_str(&content)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()))
    }

    /// 生成重现指南
    pub fn generate_reproduction_guide(&self) -> String {
        let mut guide = String::new();

        guide.push_str("# Experiment Reproduction Guide\n\n");
        guide.push_str(&format!(
            "**Generated**: {}\n\n",
            self.environment.timestamp
        ));

        guide.push_str("## Environment Setup\n\n");
        guide.push_str(&format!(
            "- **Rust Version**: {}\n",
            self.environment.rust_version
        ));
        guide.push_str(&format!("- **Target**: {}\n", self.environment.target));
        guide.push_str(&format!("- **OS**: {}\n", self.environment.os));

        if let Some(ref git) = self.environment.git_info {
            guide.push_str("\n## Git Information\n\n");
            guide.push_str(&format!("- **Commit**: {}\n", git.commit));
            guide.push_str(&format!("- **Branch**: {}\n", git.branch));
            guide.push_str(&format!("- **Dirty**: {}\n", git.dirty));
        }

        guide.push_str("\n## Running the Experiment\n\n");
        guide.push_str("```bash\n");
        guide.push_str(&format!(
            "# Set random seed\nexport CADAGENT_SEED={}\n",
            self.seed
        ));
        guide.push_str("\n# Run experiment\ncargo test --test experiment_test -- --nocapture\n");
        guide.push_str("```\n");

        if !self.experiment_params.is_empty() {
            guide.push_str("\n## Experiment Parameters\n\n");
            for (key, value) in &self.experiment_params {
                guide.push_str(&format!("- **{}**: {}\n", key, value));
            }
        }

        if !self.reproduction_notes.is_empty() {
            guide.push_str("\n## Notes\n\n");
            guide.push_str(&self.reproduction_notes);
        }

        guide
    }
}

impl Default for ReproducibilityConfig {
    fn default() -> Self {
        Self::new()
    }
}

impl EnvironmentInfo {
    /// 捕获当前环境信息
    pub fn capture() -> Self {
        let mut env_vars = HashMap::new();

        // 捕获相关环境变量
        let relevant_vars = [
            "RUST_VERSION",
            "CARGO_VERSION",
            "RUSTUP_TOOLCHAIN",
            "CARGO_HOME",
            "RUSTUP_HOME",
        ];

        for var in &relevant_vars {
            if let Ok(val) = std::env::var(var) {
                env_vars.insert(var.to_string(), val);
            }
        }

        // 获取 Git 信息
        let git_info = Self::capture_git_info();

        // 获取依赖版本
        let dependencies = Self::capture_dependencies();

        Self {
            timestamp: chrono_lite_timestamp(),
            rust_version: rustc_version(),
            cargo_version: cargo_version(),
            target: std::env::var("TARGET").unwrap_or_else(|_| "unknown".to_string()),
            host: std::env::var("HOST").unwrap_or_else(|_| "unknown".to_string()),
            num_cpus: num_cpus::get(),
            total_memory_mb: total_memory(),
            os: std::env::consts::OS.to_string(),
            env_vars,
            git_info,
            dependencies,
        }
    }

    /// 捕获 Git 信息
    fn capture_git_info() -> Option<GitInfo> {
        // 尝试从 git 命令获取信息
        let commit = std::process::Command::new("git")
            .args(["rev-parse", "HEAD"])
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .map(|s| s.trim().to_string())?;

        let branch = std::process::Command::new("git")
            .args(["rev-parse", "--abbrev-ref", "HEAD"])
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .map(|s| s.trim().to_string())
            .unwrap_or_else(|| "unknown".to_string());

        let dirty = std::process::Command::new("git")
            .args(["status", "--porcelain"])
            .output()
            .ok()
            .map(|o| !o.stdout.is_empty())
            .unwrap_or(false);

        let commit_date = std::process::Command::new("git")
            .args(["log", "-1", "--format=%ci"])
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .map(|s| s.trim().to_string())
            .unwrap_or_else(|| "unknown".to_string());

        let commit_message = std::process::Command::new("git")
            .args(["log", "-1", "--format=%s"])
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .map(|s| s.trim().to_string());

        Some(GitInfo {
            commit,
            branch,
            dirty,
            commit_date,
            commit_message,
        })
    }

    /// 捕获依赖版本
    fn capture_dependencies() -> HashMap<String, String> {
        let mut deps = HashMap::new();

        // 从 Cargo.toml 读取依赖
        if let Ok(cargo_toml) = std::fs::read_to_string("Cargo.toml") {
            let mut in_deps = false;
            for line in cargo_toml.lines() {
                if line.trim() == "[dependencies]" {
                    in_deps = true;
                    continue;
                }
                if line.starts_with('[') && line.ends_with(']') {
                    in_deps = false;
                    continue;
                }
                if in_deps && line.contains('=') {
                    let parts: Vec<&str> = line.split('=').collect();
                    if parts.len() >= 2 {
                        let name = parts[0].trim();
                        let version = parts[1].trim().trim_matches('"');
                        deps.insert(name.to_string(), version.to_string());
                    }
                }
            }
        }

        deps
    }

    /// 保存环境信息
    pub fn save_to(&self, path: &Path) -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let content = serde_json::to_string_pretty(self)?;
        fs::write(path, content)
    }

    /// 从文件加载环境信息
    pub fn load_from(path: &Path) -> std::io::Result<Self> {
        let content = fs::read_to_string(path)?;
        serde_json::from_str(&content)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()))
    }

    /// 验证环境是否匹配
    pub fn matches(&self, other: &EnvironmentInfo) -> bool {
        // 检查关键环境是否匹配
        self.rust_version == other.rust_version
            && self.target == other.target
            && self.os == other.os
    }
}

impl Default for EnvironmentInfo {
    fn default() -> Self {
        Self {
            timestamp: chrono_lite_timestamp(),
            rust_version: rustc_version(),
            cargo_version: cargo_version(),
            target: "unknown".to_string(),
            host: "unknown".to_string(),
            num_cpus: 1,
            total_memory_mb: None,
            os: "unknown".to_string(),
            env_vars: HashMap::new(),
            git_info: None,
            dependencies: HashMap::new(),
        }
    }
}

/// 获取 Rust 版本
fn rustc_version() -> String {
    std::process::Command::new("rustc")
        .arg("--version")
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "unknown".to_string())
}

/// 获取 Cargo 版本
fn cargo_version() -> String {
    std::process::Command::new("cargo")
        .arg("--version")
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "unknown".to_string())
}

/// 获取 CPU 核心数
mod num_cpus {
    pub fn get() -> usize {
        std::thread::available_parallelism()
            .map(|p| p.get())
            .unwrap_or(1)
    }
}

/// 获取总内存 (MB)
fn total_memory() -> Option<u64> {
    // 简化实现，实际应该读取系统信息
    #[cfg(target_os = "linux")]
    {
        if let Ok(meminfo) = std::fs::read_to_string("/proc/meminfo") {
            for line in meminfo.lines() {
                if line.starts_with("MemTotal:") {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 2 {
                        if let Ok(kb) = parts[1].parse::<u64>() {
                            return Some(kb / 1024);
                        }
                    }
                }
            }
        }
    }
    None
}

/// 简单的 timestamp 生成
fn chrono_lite_timestamp() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let duration = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
    let secs = duration.as_secs();
    format!("Unix timestamp: {}", secs)
}

/// 实验运行记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperimentRun {
    /// 实验名称
    pub experiment_name: String,
    /// 运行时间戳
    pub timestamp: String,
    /// 使用的种子
    pub seed: u64,
    /// 环境信息
    pub environment: EnvironmentInfo,
    /// 配置参数
    pub config: HashMap<String, String>,
    /// 运行结果
    pub result: String,
    /// 是否成功
    pub success: bool,
}

impl ExperimentRun {
    pub fn new(experiment_name: &str) -> Self {
        Self {
            experiment_name: experiment_name.to_string(),
            timestamp: chrono_lite_timestamp(),
            seed: 42,
            environment: EnvironmentInfo::capture(),
            config: HashMap::new(),
            result: String::new(),
            success: true,
        }
    }

    pub fn with_seed(mut self, seed: u64) -> Self {
        self.seed = seed;
        self
    }

    pub fn with_config(mut self, key: &str, value: &str) -> Self {
        self.config.insert(key.to_string(), value.to_string());
        self
    }

    pub fn with_result(mut self, result: &str, success: bool) -> Self {
        self.result = result.to_string();
        self.success = success;
        self
    }

    /// 保存运行记录
    pub fn save_to(&self, path: &Path) -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let content = serde_json::to_string_pretty(self)?;
        fs::write(path, content)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_seed_manager() {
        let mut manager = SeedManager::new(42);

        let seed1 = manager.next_seed();
        let seed2 = manager.next_seed();
        let seed3 = manager.next_seed();

        assert_eq!(seed1, 42);
        assert_eq!(seed2, 43);
        assert_eq!(seed3, 44);

        manager.reset();
        assert_eq!(manager.next_seed(), 42);
    }

    #[test]
    fn test_environment_capture() {
        let env = EnvironmentInfo::capture();

        assert!(!env.rust_version.is_empty());
        assert!(!env.cargo_version.is_empty());
        assert!(env.num_cpus > 0);
    }

    #[test]
    fn test_reproducibility_config() {
        let config = ReproducibilityConfig::new()
            .with_seed(123)
            .with_deterministic(true)
            .with_param("num_samples", "1000");

        assert_eq!(config.seed, 123);
        assert!(config.deterministic);
        assert_eq!(
            config.experiment_params.get("num_samples"),
            Some(&"1000".to_string())
        );
    }
}
