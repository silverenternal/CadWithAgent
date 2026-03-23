//! LLM 推理模块
//!
//! 大模型驱动的思维链推理架构
//!
//! # 模块定位
//!
//! **这是真正的 AI 思维链模块**，由 LLM 驱动推理过程：
//! - 动态生成推理步骤
//! - 支持回溯和条件分支
//! - 处理不确定性和多义性
//! - 生成可解释的推理过程
//!
//! # 与 geometry_pipeline 的关系
//!
//! | 模块 | 定位 | 特点 |
//! |------|------|------|
//! | `geometry_pipeline` | 确定性几何处理工具 | 固定流程、数学算法、可验证 |
//! | `llm_reasoning` | AI 思维链推理 | 动态生成、不确定性处理、可解释 |
//!
//! # 架构设计
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                    用户问题                                  │
//! │              "这个户型有多少个房间？"                          │
//! └─────────────────────────────────────────────────────────────┘
//!                              │
//!                              ▼
//! ┌─────────────────────────────────────────────────────────────┐
//! │                   LLM Reasoning Engine                       │
//! │  ┌───────────┐   ┌───────────┐   ┌───────────┐              │
//! │  │ 理解任务   │──▶│ 规划步骤   │──▶│ 执行推理   │              │
//! │  │ Understand │   │ Plan      │   │ Execute   │              │
//! │  └───────────┘   └───────────┘   └───────────┘              │
//! │         │               │               │                    │
//! │         │               │               ▼                    │
//! │         │               │    ┌───────────────────┐           │
//! │         │               │    │ geometry_pipeline │           │
//! │         │               │    │  (几何处理工具)    │           │
//! │         │               │    └───────────────────┘           │
//! │         │               │               │                    │
//! │         ▼               ▼               ▼                    │
//! │  ┌───────────────────────────────────────────────────┐      │
//! │  │              Chain of Thought                      │      │
//! │  │  1. 理解：这是一个房间计数任务...                   │      │
//! │  │  2. 规划：我将按以下步骤执行...                     │      │
//! │  │  3. 工具：调用 geometry_pipeline...                │      │
//! │  │  4. 分析：识别出 6 个基元，拓扑图显示...             │      │
//! │  │  5. 结论：共有 3 个房间                             │      │
//! │  └───────────────────────────────────────────────────┘      │
//! └─────────────────────────────────────────────────────────────┘
//!                              │
//!                              ▼
//! ┌─────────────────────────────────────────────────────────────┐
//! │                      最终答案                                │
//! │            "共有 3 个房间" (置信度：0.85)                     │
//! └─────────────────────────────────────────────────────────────┘
//! ```
//!
//! # 使用示例
//!
//! ```rust,no_run
//! use cadagent::llm_reasoning::{LlmReasoningEngine, LlmReasoningRequest, ReasoningTask};
//! use serde_json::json;
//!
//! let engine = LlmReasoningEngine::new();
//! let request = LlmReasoningRequest {
//!     task: "这个户型有多少个房间？".to_string(),
//!     task_type: ReasoningTask::CountRooms,
//!     context: json!({
//!         "drawing_type": "vector",
//!         "drawing_data": "vector_data_here"
//!     }),
//!     verbose: false,
//! };
//!
//! let response = engine.reason(request).unwrap();
//! println!("答案：{}", response.chain_of_thought.answer);
//! println!("置信度：{:.2}", response.chain_of_thought.confidence);
//! println!("推理步骤：{}", response.chain_of_thought.steps.len());
//! ```

pub mod types;
pub mod engine;

pub use types::*;
pub use engine::*;

use tokitai::tool;

/// LLM 推理工具（tokitai 工具封装）
#[derive(Default, Clone)]
pub struct LlmReasoningTools;

#[tool]
impl LlmReasoningTools {
    /// 执行 LLM 驱动的思维链推理
    ///
    /// # 参数
    ///
    /// * `task` - 任务描述
    /// * `task_type` - 任务类型："count_rooms", "calculate_area", "measure_dimension", 
    ///                 "detect_doors_windows", "analyze_layout", "custom"
    /// * `context` - 上下文数据（JSON 格式）
    ///
    /// # 返回
    ///
    /// 包含完整思维链和最终答案的响应
    #[tool(name = "llm_reasoning_execute")]
    pub fn execute(
        &self,
        task: String,
        task_type: String,
        context: String,
    ) -> serde_json::Value {
        let engine = LlmReasoningEngine::new();
        
        let task_type_parsed = match task_type.to_lowercase().as_str() {
            "count_rooms" => ReasoningTask::CountRooms,
            "calculate_area" => ReasoningTask::CalculateArea,
            "measure_dimension" => ReasoningTask::MeasureDimension,
            "detect_doors_windows" => ReasoningTask::DetectDoorsWindows,
            "analyze_layout" => ReasoningTask::AnalyzeLayout,
            _ => ReasoningTask::Custom,
        };

        let context_json: serde_json::Value = serde_json::from_str(&context)
            .unwrap_or_else(|_| serde_json::json!({}));

        let request = LlmReasoningRequest {
            task,
            task_type: task_type_parsed,
            context: context_json,
            verbose: false,
        };

        match engine.reason(request) {
            Ok(response) => serde_json::json!({
                "success": true,
                "answer": response.chain_of_thought.answer,
                "confidence": response.chain_of_thought.confidence,
                "steps_count": response.chain_of_thought.steps.len(),
                "tools_used": response.tools_used,
                "latency_ms": response.latency_ms,
                "chain_of_thought": response.chain_of_thought
            }),
            Err(e) => serde_json::json!({
                "success": false,
                "error": e.to_string()
            }),
        }
    }

    /// 获取推理模板信息
    #[tool(name = "llm_reasoning_get_info")]
    pub fn get_reasoning_info(&self) -> serde_json::Value {
        serde_json::json!({
            "name": "llm_driven_reasoning",
            "description": "LLM 驱动的思维链推理：动态生成推理步骤，处理不确定性",
            "type": "ai_reasoning",
            "task_types": [
                {"id": "count_rooms", "name": "房间计数", "description": "统计户型图中的房间数量"},
                {"id": "calculate_area", "name": "面积计算", "description": "计算房间或户型的面积"},
                {"id": "measure_dimension", "name": "尺寸测量", "description": "测量长度、宽度等尺寸"},
                {"id": "detect_doors_windows", "name": "门窗检测", "description": "检测门窗位置和数量"},
                {"id": "analyze_layout", "name": "户型分析", "description": "分析户型布局和类型"}
            ],
            "output_format": {
                "chain_of_thought": {
                    "task": "任务描述",
                    "steps": "推理步骤列表（动态生成）",
                    "answer": "最终答案",
                    "confidence": "置信度"
                },
                "tools_used": "使用的工具列表",
                "latency_ms": "推理耗时"
            },
            "integration": {
                "description": "LLM 作为推理引擎，调用 geometry_pipeline 等工具获取数据",
                "workflow": "理解任务 → 规划步骤 → 调用工具 → 分析结果 → 生成结论"
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_llm_reasoning_tools() {
        let tools = LlmReasoningTools::default();
        
        let result = tools.execute(
            "这个户型有多少个房间？".to_string(),
            "count_rooms".to_string(),
            r#"{"drawing_type": "vector", "drawing_data": "test"}"#.to_string(),
        );
        
        assert!(result["success"].as_bool().unwrap_or(false));
        assert!(!result["answer"].as_str().unwrap_or("").is_empty());
    }

    #[test]
    fn test_get_reasoning_info() {
        let tools = LlmReasoningTools::default();
        let info = tools.get_reasoning_info();
        
        assert_eq!(info["name"], "llm_driven_reasoning");
        assert_eq!(info["type"], "ai_reasoning");
        assert!(info["task_types"].as_array().unwrap().len() > 0);
    }
}
