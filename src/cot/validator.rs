//! `CoT` 模板认知合理性验证器
#![allow(clippy::cast_precision_loss)]
//!
//! 验证思维链模板是否符合认知科学原理和教学最佳实践

use crate::cot::templates::{PerceptionTemplate, ReasoningTemplate, SummaryTemplate};
use serde::{Deserialize, Serialize};

/// `CoT` 模板验证结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CotValidationResult {
    /// 是否通过验证
    pub is_valid: bool,
    /// 验证通过的检查项
    pub passed_checks: Vec<ValidationCheck>,
    /// 验证失败的检查项
    pub failed_checks: Vec<ValidationFailure>,
    /// 总体评分 (0-100)
    pub score: f64,
    /// 改进建议
    pub suggestions: Vec<String>,
}

/// 验证检查项
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationCheck {
    /// 检查项名称
    pub name: String,
    /// 检查项描述
    pub description: String,
    /// 是否通过
    pub passed: bool,
    /// 详细信息
    pub details: Option<String>,
}

/// 验证失败项
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationFailure {
    /// 失败项名称
    pub name: String,
    /// 失败原因
    pub reason: String,
    /// 严重程度
    pub severity: Severity,
    /// 修复建议
    pub suggestion: String,
}

/// 严重程度
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Severity {
    /// 低优先级建议
    Low,
    /// 中优先级警告
    Medium,
    /// 高优先级错误
    High,
    /// 关键错误
    Critical,
}

/// `CoT` 模板验证器
pub struct CotTemplateValidator {
    /// 认知科学原则检查列表
    #[allow(dead_code)] // Reserved for future extensibility - defines validation framework
    cognitive_principles: Vec<CognitivePrinciple>,
    /// 教学最佳实践检查列表
    #[allow(dead_code)] // Reserved for future extensibility - defines validation framework
    pedagogical_practices: Vec<PedagogicalPractice>,
}

impl Default for CotTemplateValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl CotTemplateValidator {
    /// 创建新的验证器
    pub fn new() -> Self {
        Self {
            cognitive_principles: vec![
                CognitivePrinciple::CognitiveLoad,
                CognitivePrinciple::ProgressiveDisclosure,
                CognitivePrinciple::ConcreteToAbstract,
                CognitivePrinciple::Scaffolding,
            ],
            pedagogical_practices: vec![
                PedagogicalPractice::ClearStructure,
                PedagogicalPractice::ExplicitReasoning,
                PedagogicalPractice::SelfExplanation,
                PedagogicalPractice::Metacognition,
            ],
        }
    }

    /// 验证感知模板
    pub fn validate_perception(&self, template: &PerceptionTemplate) -> CotValidationResult {
        let mut checks = Vec::new();
        let mut failures = Vec::new();
        let mut suggestions = Vec::new();

        // 检查 1: 模板长度 (认知负载)
        let template_len = template.pattern.len();
        if template_len > 200 {
            failures.push(ValidationFailure {
                name: "模板长度".to_string(),
                reason: format!("模板过长 ({template_len} 字符)，可能增加认知负载"),
                severity: Severity::Medium,
                suggestion: "建议将模板控制在 200 字符以内，或拆分为多个子模板".to_string(),
            });
        } else {
            checks.push(ValidationCheck {
                name: "认知负载".to_string(),
                description: "模板长度适中，不会造成信息过载".to_string(),
                passed: true,
                details: Some(format!("当前长度：{template_len} 字符")),
            });
        }

        // 检查 2: 占位符完整性
        let required_placeholders = ["{element_type}", "{coords}", "{vertex_count}"];
        let mut missing_placeholders = Vec::new();
        for placeholder in &required_placeholders {
            if !template.pattern.contains(placeholder) {
                missing_placeholders.push(*placeholder);
            }
        }

        if missing_placeholders.is_empty() {
            checks.push(ValidationCheck {
                name: "占位符完整性".to_string(),
                description: "模板包含所有必要的信息占位符".to_string(),
                passed: true,
                details: None,
            });
        } else {
            failures.push(ValidationFailure {
                name: "占位符完整性".to_string(),
                reason: format!("缺少必要占位符：{missing_placeholders:?}"),
                severity: Severity::High,
                suggestion: "添加所有必要的占位符以确保信息完整性".to_string(),
            });
        }

        // 检查 3: 语言清晰度
        if template.pattern.contains("可能") || template.pattern.contains("也许") {
            suggestions.push("感知阶段应使用确定性语言，避免模糊表达".to_string());
        }

        checks.push(ValidationCheck {
            name: "语言清晰度".to_string(),
            description: "感知描述应使用明确、客观的语言".to_string(),
            passed: !template.pattern.contains("可能") && !template.pattern.contains("也许"),
            details: None,
        });

        // 检查 4: 结构化程度
        let has_structure = template.pattern.contains("首先")
            || template.pattern.contains("第一步")
            || template.pattern.contains("观察");

        if has_structure {
            checks.push(ValidationCheck {
                name: "结构化引导".to_string(),
                description: "模板包含结构化引导词，有助于学生理解步骤".to_string(),
                passed: true,
                details: None,
            });
        } else {
            suggestions.push("考虑添加结构化引导词（如'首先'、'观察'）以增强教学引导".to_string());
        }

        // 计算评分
        let passed_count = checks.iter().filter(|c| c.passed).count();
        let total_checks = checks.len();
        let base_score = if total_checks > 0 {
            (passed_count as f64 / total_checks as f64) * 100.0
        } else {
            0.0
        };

        // 根据失败严重程度扣分
        let penalty: f64 = failures
            .iter()
            .map(|f| match f.severity {
                Severity::Critical => 25.0,
                Severity::High => 15.0,
                Severity::Medium => 8.0,
                Severity::Low => 3.0,
            })
            .sum();

        let final_score = (base_score - penalty).clamp(0.0, 100.0);

        CotValidationResult {
            is_valid: failures.is_empty() || final_score >= 60.0,
            passed_checks: checks,
            failed_checks: failures,
            score: final_score,
            suggestions,
        }
    }

    /// 验证推理模板
    pub fn validate_reasoning(&self, template: &ReasoningTemplate) -> CotValidationResult {
        let mut checks = Vec::new();
        let mut failures = Vec::new();
        let mut suggestions = Vec::new();

        // 检查 1: 推理链完整性 (前提→过程→结论)
        let has_rule = template.pattern.contains("{rule}");
        let has_observation = template.pattern.contains("{observation}");
        let has_conclusion = template.pattern.contains("{conclusion}");

        if has_rule && has_observation && has_conclusion {
            checks.push(ValidationCheck {
                name: "推理链完整性".to_string(),
                description: "模板包含完整的推理链：规则→观察→结论".to_string(),
                passed: true,
                details: None,
            });
        } else {
            let missing: Vec<_> = [
                (!has_rule).then_some("rule"),
                (!has_observation).then_some("observation"),
                (!has_conclusion).then_some("conclusion"),
            ]
            .into_iter()
            .flatten()
            .collect();

            failures.push(ValidationFailure {
                name: "推理链完整性".to_string(),
                reason: format!("缺少推理链组件：{missing:?}"),
                severity: Severity::High,
                suggestion: "确保模板包含规则、观察和结论三个部分".to_string(),
            });
        }

        // 检查 2: 逻辑连接词
        let has_logical_connectors = template.pattern.contains("根据")
            || template.pattern.contains("因为")
            || template.pattern.contains("所以")
            || template.pattern.contains("因此");

        if has_logical_connectors {
            checks.push(ValidationCheck {
                name: "逻辑连接".to_string(),
                description: "使用逻辑连接词明确推理关系".to_string(),
                passed: true,
                details: None,
            });
        } else {
            suggestions.push("添加逻辑连接词（如'根据'、'因此'）以明确推理关系".to_string());
        }

        // 检查 3: 元认知提示
        let has_metacognition = template.pattern.contains("分析")
            || template.pattern.contains("思考")
            || template.pattern.contains("考虑");

        checks.push(ValidationCheck {
            name: "元认知提示".to_string(),
            description: "包含促进元认知的词汇".to_string(),
            passed: has_metacognition,
            details: None,
        });

        if !has_metacognition {
            suggestions.push("考虑添加元认知提示词（如'分析'、'思考'）以促进学生反思".to_string());
        }

        // 检查 4: 模板长度
        let template_len = template.pattern.len();
        if template_len > 150 {
            failures.push(ValidationFailure {
                name: "模板长度".to_string(),
                reason: format!("推理模板过长 ({template_len} 字符)"),
                severity: Severity::Low,
                suggestion: "推理步骤应简洁明了，建议控制在 150 字符以内".to_string(),
            });
        }

        // 计算评分
        let passed_count = checks.iter().filter(|c| c.passed).count();
        let total_checks = checks.len();
        let base_score = if total_checks > 0 {
            (passed_count as f64 / total_checks as f64) * 100.0
        } else {
            0.0
        };

        let penalty: f64 = failures
            .iter()
            .map(|f| match f.severity {
                Severity::Critical => 25.0,
                Severity::High => 15.0,
                Severity::Medium => 8.0,
                Severity::Low => 3.0,
            })
            .sum();

        let final_score = (base_score - penalty).clamp(0.0, 100.0);

        CotValidationResult {
            is_valid: failures.is_empty() || final_score >= 60.0,
            passed_checks: checks,
            failed_checks: failures,
            score: final_score,
            suggestions,
        }
    }

    /// 验证总结模板
    pub fn validate_summary(&self, template: &SummaryTemplate) -> CotValidationResult {
        let mut checks = Vec::new();
        let mut failures = Vec::new();
        let mut suggestions = Vec::new();

        // 检查 1: 总结性语言
        let has_summary_marker = template.pattern.contains("综上")
            || template.pattern.contains("总之")
            || template.pattern.contains("总结")
            || template.pattern.contains("因此");

        if has_summary_marker {
            checks.push(ValidationCheck {
                name: "总结性标记".to_string(),
                description: "使用总结性语言标记结论阶段".to_string(),
                passed: true,
                details: None,
            });
        } else {
            failures.push(ValidationFailure {
                name: "总结性标记".to_string(),
                reason: "模板缺少总结性语言标记".to_string(),
                severity: Severity::Medium,
                suggestion: "添加'综上所述'、'总之'等总结性标记词".to_string(),
            });
        }

        // 检查 2: 占位符
        if template.pattern.contains("{summary}") {
            checks.push(ValidationCheck {
                name: "内容占位符".to_string(),
                description: "包含总结内容占位符".to_string(),
                passed: true,
                details: None,
            });
        } else {
            failures.push(ValidationFailure {
                name: "内容占位符".to_string(),
                reason: "缺少总结内容占位符".to_string(),
                severity: Severity::High,
                suggestion: "添加 {summary} 占位符以插入具体总结内容".to_string(),
            });
        }

        // 检查 3: 简洁性
        let template_len = template.pattern.len();
        if template_len > 100 {
            suggestions.push("总结应简洁有力，建议控制在 100 字符以内".to_string());
        } else {
            checks.push(ValidationCheck {
                name: "简洁性".to_string(),
                description: "总结模板简洁明了".to_string(),
                passed: true,
                details: Some(format!("当前长度：{template_len} 字符")),
            });
        }

        // 计算评分
        let passed_count = checks.iter().filter(|c| c.passed).count();
        let total_checks = checks.len();
        let base_score = if total_checks > 0 {
            (passed_count as f64 / total_checks as f64) * 100.0
        } else {
            0.0
        };

        let penalty: f64 = failures
            .iter()
            .map(|f| match f.severity {
                Severity::Critical => 25.0,
                Severity::High => 15.0,
                Severity::Medium => 8.0,
                Severity::Low => 3.0,
            })
            .sum();

        let final_score = (base_score - penalty).clamp(0.0, 100.0);

        CotValidationResult {
            is_valid: failures.is_empty() || final_score >= 60.0,
            passed_checks: checks,
            failed_checks: failures,
            score: final_score,
            suggestions,
        }
    }

    /// 验证完整的 `CoT` 模板集
    pub fn validate_all(
        &self,
        perception: &PerceptionTemplate,
        reasoning: &ReasoningTemplate,
        summary: &SummaryTemplate,
    ) -> CotValidationResult {
        let perception_result = self.validate_perception(perception);
        let reasoning_result = self.validate_reasoning(reasoning);
        let summary_result = self.validate_summary(summary);

        let all_checks = [
            perception_result.passed_checks.clone(),
            reasoning_result.passed_checks.clone(),
            summary_result.passed_checks.clone(),
        ]
        .concat();

        let all_failures = [
            perception_result.failed_checks.clone(),
            reasoning_result.failed_checks.clone(),
            summary_result.failed_checks.clone(),
        ]
        .concat();

        let all_suggestions = [
            perception_result.suggestions.clone(),
            reasoning_result.suggestions.clone(),
            summary_result.suggestions.clone(),
        ]
        .concat();

        let avg_score =
            (perception_result.score + reasoning_result.score + summary_result.score) / 3.0;

        CotValidationResult {
            is_valid: perception_result.is_valid
                && reasoning_result.is_valid
                && summary_result.is_valid,
            passed_checks: all_checks,
            failed_checks: all_failures,
            score: avg_score,
            suggestions: all_suggestions,
        }
    }
}

/// 认知科学原则
#[derive(Debug, Clone, Copy)]
enum CognitivePrinciple {
    /// 认知负载理论 - 避免信息过载
    CognitiveLoad,
    /// 渐进式披露 - 信息分步呈现
    ProgressiveDisclosure,
    /// 从具体到抽象 - 先感知后推理
    ConcreteToAbstract,
    /// 支架式教学 - 提供适当引导
    Scaffolding,
}

/// 教学最佳实践
#[derive(Debug, Clone, Copy)]
enum PedagogicalPractice {
    /// 清晰的结构
    ClearStructure,
    /// 显式推理
    ExplicitReasoning,
    /// 自我解释
    SelfExplanation,
    /// 元认知
    Metacognition,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validator_new() {
        let validator = CotTemplateValidator::new();
        assert!(!validator.cognitive_principles.is_empty());
        assert!(!validator.pedagogical_practices.is_empty());
    }

    #[test]
    fn test_validate_perception_valid() {
        let validator = CotTemplateValidator::new();
        let template = PerceptionTemplate::default();

        let result = validator.validate_perception(&template);

        assert!(result.is_valid);
        assert!(result.score > 0.0);
        assert!(!result.passed_checks.is_empty());
    }

    #[test]
    fn test_validate_perception_too_long() {
        let validator = CotTemplateValidator::new();
        let template = PerceptionTemplate {
            pattern: "这是一个非常非常长的模板，".repeat(30),
        };

        let result = validator.validate_perception(&template);

        assert!(result.failed_checks.iter().any(|f| f.name == "模板长度"));
    }

    #[test]
    fn test_validate_reasoning_valid() {
        let validator = CotTemplateValidator::new();
        let template = ReasoningTemplate::default();

        let result = validator.validate_reasoning(&template);

        assert!(result.is_valid);
        assert!(result.score > 0.0);
    }

    #[test]
    fn test_validate_reasoning_missing_components() {
        let validator = CotTemplateValidator::new();
        let template = ReasoningTemplate {
            pattern: "这是一个不完整的推理模板".to_string(),
        };

        let result = validator.validate_reasoning(&template);

        assert!(result
            .failed_checks
            .iter()
            .any(|f| f.name == "推理链完整性"));
    }

    #[test]
    fn test_validate_summary_valid() {
        let validator = CotTemplateValidator::new();
        let template = SummaryTemplate::default();

        let result = validator.validate_summary(&template);

        assert!(result.is_valid);
        assert!(result.passed_checks.iter().any(|c| c.name == "总结性标记"));
    }

    #[test]
    fn test_validate_summary_missing_marker() {
        let validator = CotTemplateValidator::new();
        let template = SummaryTemplate {
            pattern: "{summary}".to_string(),
        };

        let result = validator.validate_summary(&template);

        assert!(result.failed_checks.iter().any(|f| f.name == "总结性标记"));
    }

    #[test]
    fn test_validate_all() {
        let validator = CotTemplateValidator::new();
        let perception = PerceptionTemplate::default();
        let reasoning = ReasoningTemplate::default();
        let summary = SummaryTemplate::default();

        let result = validator.validate_all(&perception, &reasoning, &summary);

        assert!(result.is_valid);
        assert!(result.score > 0.0);
        assert!(result.passed_checks.len() >= 3);
    }

    #[test]
    fn test_validation_result_serialization() {
        let result = CotValidationResult {
            is_valid: true,
            passed_checks: vec![ValidationCheck {
                name: "测试检查".to_string(),
                description: "测试描述".to_string(),
                passed: true,
                details: Some("详细信息".to_string()),
            }],
            failed_checks: vec![],
            score: 85.0,
            suggestions: vec!["建议 1".to_string()],
        };

        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("is_valid"));
        assert!(json.contains("score"));

        let deserialized: CotValidationResult = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.is_valid, result.is_valid);
        assert_eq!(deserialized.score, result.score);
    }

    #[test]
    fn test_severity_serialization() {
        let severities = [
            Severity::Low,
            Severity::Medium,
            Severity::High,
            Severity::Critical,
        ];

        for severity in &severities {
            let json = serde_json::to_string(severity).unwrap();
            let deserialized: Severity = serde_json::from_str(&json).unwrap();
            assert_eq!(*severity, deserialized);
        }
    }

    #[test]
    fn test_validation_failure() {
        let failure = ValidationFailure {
            name: "测试失败".to_string(),
            reason: "测试原因".to_string(),
            severity: Severity::High,
            suggestion: "测试建议".to_string(),
        };

        assert_eq!(failure.severity, Severity::High);
        assert!(!failure.suggestion.is_empty());
    }

    #[test]
    fn test_cognitive_principle_enum() {
        let principles = [
            CognitivePrinciple::CognitiveLoad,
            CognitivePrinciple::ProgressiveDisclosure,
            CognitivePrinciple::ConcreteToAbstract,
            CognitivePrinciple::Scaffolding,
        ];

        assert_eq!(principles.len(), 4);
    }

    #[test]
    fn test_pedagogical_practice_enum() {
        let practices = [
            PedagogicalPractice::ClearStructure,
            PedagogicalPractice::ExplicitReasoning,
            PedagogicalPractice::SelfExplanation,
            PedagogicalPractice::Metacognition,
        ];

        assert_eq!(practices.len(), 4);
    }

    #[test]
    fn test_validate_default_templates() {
        use crate::cot::templates::{PerceptionTemplate, ReasoningTemplate, SummaryTemplate};

        let validator = CotTemplateValidator::new();
        let perception = PerceptionTemplate::default();
        let reasoning = ReasoningTemplate::default();
        let summary = SummaryTemplate::default();

        let perception_result = validator.validate_perception(&perception);
        let reasoning_result = validator.validate_reasoning(&reasoning);
        let summary_result = validator.validate_summary(&summary);

        // 验证默认模板应该基本合理
        assert!(perception_result.is_valid, "默认感知模板应通过验证");
        assert!(reasoning_result.is_valid, "默认推理模板应通过验证");
        assert!(summary_result.is_valid, "默认总结模板应通过验证");

        // 验证评分应该在合理范围内
        assert!(perception_result.score >= 60.0, "感知模板评分应 >= 60");
        assert!(reasoning_result.score >= 60.0, "推理模板评分应 >= 60");
        assert!(summary_result.score >= 60.0, "总结模板评分应 >= 60");

        // 验证总体验证
        let overall_result = validator.validate_all(&perception, &reasoning, &summary);
        assert!(overall_result.is_valid, "默认模板集应通过整体验证");
        assert!(overall_result.score >= 60.0, "整体评分应 >= 60");
    }
}
