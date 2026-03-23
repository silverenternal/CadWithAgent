//! JSON 导出器
//!
//! 将几何图元导出为结构化 JSON 格式

use crate::geometry::{Primitive, Room};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::Write;
use std::path::Path;

/// JSON 导出器
pub struct JsonExporter;

impl JsonExporter {
    /// 导出图元到 JSON 文件
    pub fn export(primitives: &[Primitive], output_path: impl AsRef<Path>) -> Result<JsonExportResult, JsonExportError> {
        let json_str = serde_json::to_string_pretty(primitives)?;
        let mut file = File::create(output_path.as_ref())?;
        file.write_all(json_str.as_bytes())?;

        Ok(JsonExportResult {
            success: true,
            path: output_path.as_ref().to_string_lossy().to_string(),
            entity_count: primitives.len(),
        })
    }

    /// 导出为带 Geo-CoT 标签的 JSON
    pub fn export_with_cot(
        primitives: &[Primitive],
        thinking: &str,
        answer: &str,
        output_path: impl AsRef<Path>,
    ) -> Result<JsonExportResult, JsonExportError> {
        let cot_data = CotExportData {
            primitives: primitives.to_vec(),
            thinking: thinking.to_string(),
            answer: answer.to_string(),
        };

        let json_str = serde_json::to_string_pretty(&cot_data)?;
        let mut file = File::create(output_path.as_ref())?;
        file.write_all(json_str.as_bytes())?;

        Ok(JsonExportResult {
            success: true,
            path: output_path.as_ref().to_string_lossy().to_string(),
            entity_count: primitives.len(),
        })
    }

    /// 导出房间数据
    pub fn export_rooms(rooms: &[Room], output_path: impl AsRef<Path>) -> Result<JsonExportResult, JsonExportError> {
        let json_str = serde_json::to_string_pretty(rooms)?;
        let mut file = File::create(output_path.as_ref())?;
        file.write_all(json_str.as_bytes())?;

        Ok(JsonExportResult {
            success: true,
            path: output_path.as_ref().to_string_lossy().to_string(),
            entity_count: rooms.len(),
        })
    }

    /// 导出为训练数据格式（多模态指令微调）
    pub fn export_training_data(
        image_path: &str,
        primitives: &[Primitive],
        instruction: &str,
        thinking: &str,
        answer: &str,
        output_path: impl AsRef<Path>,
    ) -> Result<JsonExportResult, JsonExportError> {
        let training_data = TrainingData {
            image: image_path.to_string(),
            instruction: instruction.to_string(),
            grounding: GroundTruth {
                primitives: primitives.to_vec(),
            },
            thinking: thinking.to_string(),
            answer: answer.to_string(),
        };

        let json_str = serde_json::to_string_pretty(&training_data)?;
        let mut file = File::create(output_path.as_ref())?;
        file.write_all(json_str.as_bytes())?;

        Ok(JsonExportResult {
            success: true,
            path: output_path.as_ref().to_string_lossy().to_string(),
            entity_count: 1,
        })
    }
}

/// JSON 导出结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonExportResult {
    pub success: bool,
    pub path: String,
    pub entity_count: usize,
}

/// JSON 导出错误
#[derive(Debug, thiserror::Error)]
pub enum JsonExportError {
    #[error("文件写入失败：{0}")]
    IoError(#[from] std::io::Error),

    #[error("JSON 序列化失败：{0}")]
    JsonError(#[from] serde_json::Error),
}

/// 带 Geo-CoT 的导出数据结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CotExportData {
    pub primitives: Vec<Primitive>,
    pub thinking: String,
    pub answer: String,
}

/// 训练数据结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingData {
    pub image: String,
    pub instruction: String,
    pub grounding: GroundTruth,
    pub thinking: String,
    pub answer: String,
}

/// 真实标注数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroundTruth {
    pub primitives: Vec<Primitive>,
}

/// QA 对数据结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QAPair {
    pub question: String,
    pub answer: String,
    pub thinking: Option<String>,
}

/// 导出 QA 数据集
pub fn export_qa_dataset(
    qa_pairs: &[QAPair],
    output_path: impl AsRef<Path>,
) -> Result<JsonExportResult, JsonExportError> {
    let json_str = serde_json::to_string_pretty(qa_pairs)?;
    let mut file = File::create(output_path.as_ref())?;
    file.write_all(json_str.as_bytes())?;

    Ok(JsonExportResult {
        success: true,
        path: output_path.as_ref().to_string_lossy().to_string(),
        entity_count: qa_pairs.len(),
    })
}
