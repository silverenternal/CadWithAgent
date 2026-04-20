//! 有效性威胁分析模块
//!
//! 提供系统化的有效性威胁识别、分类和缓解策略，符合顶会论文要求。
//!
//! # 使用示例
//!
//! ```rust
//! use experiment::validity_threats::{
//!     ThreatAnalyzer, ValidityThreat, ThreatCategory, ThreatSeverity
//! };
//!
//! // 创建威胁分析器
//! let mut analyzer = ThreatAnalyzer::new("VLM Reasoning Quality Experiment");
//!
//! // 添加内部有效性威胁
//! analyzer.add_threat(ValidityThreat::new(
//!     "Selection Bias",
//!     ThreatCategory::Internal,
//!     ThreatSeverity::Medium,
//!     "测试样本可能不具有代表性",
//!     "使用随机抽样和固定种子确保可重复性",
//! ));
//!
//! // 生成分析报告
//! let report = analyzer.generate_report();
//! println!("{}", report.to_markdown());
//! ```

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 威胁类别
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ThreatCategory {
    /// 内部有效性 - 实验执行是否严谨
    Internal,
    /// 外部有效性 - 结果是否可推广
    External,
    /// 结构有效性 - 测量工具是否有效
    Construct,
    /// 结论有效性 - 统计结论是否正确
    Conclusion,
}

impl ThreatCategory {
    pub fn as_str(&self) -> &'static str {
        match self {
            ThreatCategory::Internal => "Internal Validity",
            ThreatCategory::External => "External Validity",
            ThreatCategory::Construct => "Construct Validity",
            ThreatCategory::Conclusion => "Conclusion Validity",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            ThreatCategory::Internal => {
                "Concerns about whether the experiment was conducted properly"
            }
            ThreatCategory::External => "Concerns about generalizability of results",
            ThreatCategory::Construct => {
                "Concerns about whether measures actually measure what they intend to"
            }
            ThreatCategory::Conclusion => "Concerns about statistical conclusions",
        }
    }
}

/// 威胁严重程度
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ThreatSeverity {
    /// 低 - 影响很小
    Low,
    /// 中 - 有一定影响
    Medium,
    /// 高 - 严重影响
    High,
    /// 严重 - 可能使结论无效
    Critical,
}

impl ThreatSeverity {
    pub fn as_str(&self) -> &'static str {
        match self {
            ThreatSeverity::Low => "Low",
            ThreatSeverity::Medium => "Medium",
            ThreatSeverity::High => "High",
            ThreatSeverity::Critical => "Critical",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            ThreatSeverity::Low => "◔",
            ThreatSeverity::Medium => "◑",
            ThreatSeverity::High => "◕",
            ThreatSeverity::Critical => "●",
        }
    }
}

/// 有效性威胁
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidityThreat {
    /// 威胁名称
    pub name: String,
    /// 威胁类别
    pub category: ThreatCategory,
    /// 严重程度
    pub severity: ThreatSeverity,
    /// 威胁描述
    pub description: String,
    /// 缓解策略
    pub mitigation: String,
    /// 剩余风险
    pub residual_risk: String,
}

impl ValidityThreat {
    pub fn new(
        name: &str,
        category: ThreatCategory,
        severity: ThreatSeverity,
        description: &str,
        mitigation: &str,
    ) -> Self {
        Self {
            name: name.to_string(),
            category,
            severity,
            description: description.to_string(),
            mitigation: mitigation.to_string(),
            residual_risk: String::new(),
        }
    }

    pub fn with_residual_risk(mut self, residual_risk: &str) -> Self {
        self.residual_risk = residual_risk.to_string();
        self
    }
}

/// 威胁分析报告
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreatAnalysisReport {
    /// 实验名称
    pub experiment_name: String,
    /// 分析日期
    pub analysis_date: String,
    /// 所有威胁
    pub threats: Vec<ValidityThreat>,
    /// 按类别汇总
    pub summary_by_category: HashMap<String, CategorySummary>,
    /// 总体风险评估
    pub overall_risk_assessment: String,
}

impl ThreatAnalysisReport {
    pub fn to_markdown(&self) -> String {
        let mut md = String::new();

        md.push_str(&format!(
            "# Validity Threats Analysis: {}\n\n",
            self.experiment_name
        ));
        md.push_str(&format!("**Analysis Date**: {}\n\n", self.analysis_date));

        // 总体风险评估
        md.push_str("## Overall Risk Assessment\n\n");
        md.push_str(&format!("{}\n\n", self.overall_risk_assessment));

        // 按类别展示威胁
        for (category_name, summary) in &self.summary_by_category {
            md.push_str(&format!(
                "## {} ({} threats)\n\n",
                category_name, summary.threat_count
            ));
            md.push_str(&format!(
                "**Risk Level**: {} {}\n\n",
                summary.icon(),
                summary.risk_level
            ));

            // 找出该类别的威胁
            let category_threats: Vec<_> = self
                .threats
                .iter()
                .filter(|t| t.category.as_str() == category_name)
                .collect();

            for threat in category_threats {
                md.push_str(&format!(
                    "### {} {} ({})\n\n",
                    threat.severity.icon(),
                    threat.name,
                    threat.severity.as_str()
                ));

                md.push_str(&format!("**Description**: {}\n\n", threat.description));
                md.push_str(&format!("**Mitigation**: {}\n\n", threat.mitigation));

                if !threat.residual_risk.is_empty() {
                    md.push_str(&format!("**Residual Risk**: {}\n\n", threat.residual_risk));
                }
            }
        }

        // 汇总表格
        md.push_str("## Summary Table\n\n");
        md.push_str("| Category | Count | Low | Medium | High | Critical |\n");
        md.push_str("|----------|-------|-----|--------|------|----------|\n");

        for (category_name, summary) in &self.summary_by_category {
            md.push_str(&format!(
                "| {} | {} | {} | {} | {} | {} |\n",
                category_name,
                summary.threat_count,
                summary.by_severity.get("Low").unwrap_or(&0),
                summary.by_severity.get("Medium").unwrap_or(&0),
                summary.by_severity.get("High").unwrap_or(&0),
                summary.by_severity.get("Critical").unwrap_or(&0),
            ));
        }

        md
    }

    pub fn to_latex(&self) -> String {
        let mut latex = String::new();

        latex.push_str("\\section{Validity Threats}\n\n");
        latex.push_str(&format!(
            "\\textbf{{Experiment}}: {}\n\n",
            self.experiment_name
        ));

        // 汇总表格
        latex.push_str("\\begin{table}[h]\n");
        latex.push_str("\\centering\n");
        latex.push_str("\\caption{Summary of Validity Threats}\n");
        latex.push_str("\\label{tab:validity_threats}\n");
        latex.push_str("\\begin{tabular}{lcccc}\n");
        latex.push_str("\\toprule\n");
        latex.push_str("\\textbf{Category} & \\textbf{Total} & \\textbf{Low} & \\textbf{Medium} & \\textbf{High/Critical} \\\\\n");
        latex.push_str("\\midrule\n");

        for (category_name, summary) in &self.summary_by_category {
            let high_critical = summary.by_severity.get("High").unwrap_or(&0)
                + summary.by_severity.get("Critical").unwrap_or(&0);
            latex.push_str(&format!(
                "{} & {} & {} & {} & {} \\\\\n",
                category_name,
                summary.threat_count,
                summary.by_severity.get("Low").unwrap_or(&0),
                summary.by_severity.get("Medium").unwrap_or(&0),
                high_critical,
            ));
        }

        latex.push_str("\\bottomrule\n");
        latex.push_str("\\end{tabular}\n");
        latex.push_str("\\end{table}\n\n");

        latex
    }

    pub fn save_markdown(&self, path: &std::path::Path) -> std::io::Result<()> {
        std::fs::write(path, self.to_markdown())
    }

    pub fn save_latex(&self, path: &std::path::Path) -> std::io::Result<()> {
        std::fs::write(path, self.to_latex())
    }
}

/// 类别汇总
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategorySummary {
    /// 威胁数量
    pub threat_count: usize,
    /// 按严重程度分类
    pub by_severity: HashMap<String, usize>,
    /// 风险等级描述
    pub risk_level: String,
}

impl CategorySummary {
    pub fn new() -> Self {
        Self {
            threat_count: 0,
            by_severity: HashMap::new(),
            risk_level: String::new(),
        }
    }

    pub fn icon(&self) -> &'static str {
        if self.by_severity.get("Critical").unwrap_or(&0) > &0 {
            "●"
        } else if self.by_severity.get("High").unwrap_or(&0) > &0 {
            "◕"
        } else if self.by_severity.get("Medium").unwrap_or(&0) > &0 {
            "◑"
        } else {
            "◔"
        }
    }
}

impl Default for CategorySummary {
    fn default() -> Self {
        Self::new()
    }
}

/// 威胁分析器
pub struct ThreatAnalyzer {
    experiment_name: String,
    threats: Vec<ValidityThreat>,
    common_threats: HashMap<ThreatCategory, Vec<(&'static str, &'static str, &'static str)>>,
}

impl ThreatAnalyzer {
    pub fn new(experiment_name: &str) -> Self {
        let mut analyzer = Self {
            experiment_name: experiment_name.to_string(),
            threats: Vec::new(),
            common_threats: HashMap::new(),
        };

        analyzer.init_common_threats();
        analyzer
    }

    /// 初始化常见威胁模板
    fn init_common_threats(&mut self) {
        // 内部有效性威胁
        self.common_threats.insert(
            ThreatCategory::Internal,
            vec![
                (
                    "Selection Bias",
                    "测试样本可能不具有代表性，导致结果偏差",
                    "使用随机抽样、分层抽样，确保样本多样性",
                ),
                (
                    "Testing Effects",
                    "多次测试可能导致学习效应或疲劳效应",
                    "平衡测试顺序，使用不同的测试样本",
                ),
                (
                    "Instrumentation",
                    "测量工具可能不准确或存在偏差",
                    "使用标准化工具，进行校准和验证",
                ),
                (
                    "Confounding Variables",
                    "未控制的变量可能影响结果",
                    "识别并控制潜在的混淆变量",
                ),
            ],
        );

        // 外部有效性威胁
        self.common_threats.insert(
            ThreatCategory::External,
            vec![
                (
                    "Limited Generalizability",
                    "实验结果可能无法推广到其他场景",
                    "在多样化的数据集和场景下验证",
                ),
                (
                    "Artificial Setting",
                    "实验室环境可能与实际应用不同",
                    "进行真实场景的案例研究",
                ),
                (
                    "Sample Representativeness",
                    "测试样本可能不代表总体分布",
                    "使用大规模、多样化的测试数据集",
                ),
            ],
        );

        // 结构有效性威胁
        self.common_threats.insert(
            ThreatCategory::Construct,
            vec![
                (
                    "Metric Validity",
                    "评估指标可能无法准确反映目标构念",
                    "使用领域标准指标，进行多指标评估",
                ),
                (
                    "Monomethod Bias",
                    "单一评估方法可能存在偏差",
                    "使用多种评估方法 (定量 + 定性)",
                ),
                (
                    "Construct Confounding",
                    "不同构念之间可能存在混淆",
                    "明确定义各构念，使用验证性因子分析",
                ),
            ],
        );

        // 结论有效性威胁
        self.common_threats.insert(
            ThreatCategory::Conclusion,
            vec![
                (
                    "Low Statistical Power",
                    "样本量不足可能导致统计检验力不够",
                    "进行功效分析，确保足够的样本量",
                ),
                (
                    "Violated Assumptions",
                    "统计检验的假设可能不成立",
                    "检验假设条件，使用非参数方法作为补充",
                ),
                (
                    "Fishing/p-hacking",
                    "多重比较可能增加假阳性风险",
                    "使用 Bonferroni 等校正方法，预注册分析计划",
                ),
                (
                    "Unreliable Measures",
                    "测量可能不可靠",
                    "报告内部一致性信度 (如 Cronbach's α)",
                ),
            ],
        );
    }

    /// 添加威胁
    pub fn add_threat(&mut self, threat: ValidityThreat) {
        self.threats.push(threat);
    }

    /// 从模板添加常见威胁
    pub fn add_common_threat(
        &mut self,
        category: ThreatCategory,
        threat_index: usize,
        custom_mitigation: Option<&str>,
    ) {
        if let Some(templates) = self.common_threats.get(&category) {
            if let Some(&(name, description, default_mitigation)) = templates.get(threat_index) {
                let threat = ValidityThreat::new(
                    name,
                    category,
                    ThreatSeverity::Medium,
                    description,
                    custom_mitigation.unwrap_or(default_mitigation),
                );
                self.threats.push(threat);
            }
        }
    }

    /// 生成分析报告
    pub fn generate_report(&self) -> ThreatAnalysisReport {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let mut summary_by_category = HashMap::new();

        for threat in &self.threats {
            let category_name = threat.category.as_str();
            let summary = summary_by_category
                .entry(category_name.to_string())
                .or_insert_with(CategorySummary::new);

            summary.threat_count += 1;
            *summary
                .by_severity
                .entry(threat.severity.as_str().to_string())
                .or_insert(0) += 1;

            // 计算风险等级
            summary.risk_level = if summary.by_severity.get("Critical").unwrap_or(&0) > &0 {
                "Critical".to_string()
            } else if summary.by_severity.get("High").unwrap_or(&0) > &0 {
                "High".to_string()
            } else if summary.by_severity.get("Medium").unwrap_or(&0) > &0 {
                "Medium".to_string()
            } else {
                "Low".to_string()
            };
        }

        // 总体风险评估
        let critical_count = self
            .threats
            .iter()
            .filter(|t| t.severity == ThreatSeverity::Critical)
            .count();
        let high_count = self
            .threats
            .iter()
            .filter(|t| t.severity == ThreatSeverity::High)
            .count();

        let overall_risk_assessment = if critical_count > 0 {
            format!("{} 个严重威胁需要立即解决。", critical_count)
        } else if high_count > 0 {
            format!("{} 个高严重性威胁需要在论文中讨论缓解策略。", high_count)
        } else {
            "所有威胁均已识别并制定了缓解策略，整体风险可控。".to_string()
        };

        ThreatAnalysisReport {
            experiment_name: self.experiment_name.clone(),
            analysis_date: format!("Unix timestamp: {}", timestamp),
            threats: self.threats.clone(),
            summary_by_category,
            overall_risk_assessment,
        }
    }

    /// 获取所有威胁
    pub fn threats(&self) -> &[ValidityThreat] {
        &self.threats
    }

    /// 获取威胁数量
    pub fn threat_count(&self) -> usize {
        self.threats.len()
    }
}

/// 预定义的威胁模板库
pub mod threat_templates {
    use super::*;

    /// 获取 VLM 实验的常见威胁
    pub fn vlm_experiment_threats() -> Vec<ValidityThreat> {
        vec![
            ValidityThreat::new(
                "API Variability",
                ThreatCategory::Internal,
                ThreatSeverity::Medium,
                "VLM API 可能随时间变化，影响结果可重复性",
                "记录 API 版本，使用固定版本的 API，缓存响应",
            ),
            ValidityThreat::new(
                "Prompt Sensitivity",
                ThreatCategory::Construct,
                ThreatSeverity::Medium,
                "VLM 对提示词措辞可能敏感",
                "使用标准化提示词模板，进行敏感性分析",
            ),
            ValidityThreat::new(
                "Model Hallucination",
                ThreatCategory::Conclusion,
                ThreatSeverity::High,
                "VLM 可能产生幻觉性回答",
                "使用工具增强验证，人工审核关键结果",
            ),
            ValidityThreat::new(
                "Limited Model Diversity",
                ThreatCategory::External,
                ThreatSeverity::Medium,
                "仅测试单一 VLM 可能限制结果推广性",
                "在多个 VLM 上验证，报告模型特异性",
            ),
        ]
    }

    /// 获取性能实验的常见威胁
    pub fn performance_experiment_threats() -> Vec<ValidityThreat> {
        vec![
            ValidityThreat::new(
                "Hardware Variability",
                ThreatCategory::Internal,
                ThreatSeverity::Low,
                "不同硬件配置可能影响性能测量",
                "在标准化硬件上测试，报告详细配置",
            ),
            ValidityThreat::new(
                "Warm-up Effects",
                ThreatCategory::Internal,
                ThreatSeverity::Low,
                "未充分预热可能导致性能测量偏差",
                "执行预热运行，丢弃前 N 次测量",
            ),
            ValidityThreat::new(
                "System Load",
                ThreatCategory::Internal,
                ThreatSeverity::Medium,
                "系统负载波动可能影响性能",
                "在空闲系统上测试，多次测量取中位数",
            ),
        ]
    }

    /// 获取用户研究实验的常见威胁
    pub fn user_study_threats() -> Vec<ValidityThreat> {
        vec![
            ValidityThreat::new(
                "Participant Bias",
                ThreatCategory::Internal,
                ThreatSeverity::Medium,
                "参与者可能有社会期望偏差",
                "匿名调查，强调诚实回答的重要性",
            ),
            ValidityThreat::new(
                "Learning Effects",
                ThreatCategory::Internal,
                ThreatSeverity::Medium,
                "多次任务可能导致学习效应",
                "平衡任务顺序，使用拉丁方设计",
            ),
            ValidityThreat::new(
                "Small Sample Size",
                ThreatCategory::Conclusion,
                ThreatSeverity::High,
                "参与者数量少可能限制统计效力",
                "进行功效分析确定样本量，报告效应量",
            ),
            ValidityThreat::new(
                "Demographic Limitations",
                ThreatCategory::External,
                ThreatSeverity::Medium,
                "参与者群体可能不具有代表性",
                "多样化招募，报告参与者特征",
            ),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_threat_analyzer() {
        let mut analyzer = ThreatAnalyzer::new("Test Experiment");

        analyzer.add_threat(ValidityThreat::new(
            "Test Threat",
            ThreatCategory::Internal,
            ThreatSeverity::Medium,
            "This is a test threat",
            "This is the mitigation",
        ));

        let report = analyzer.generate_report();

        assert_eq!(report.threats.len(), 1);
        assert_eq!(report.threats[0].name, "Test Threat");
        assert_eq!(report.threats[0].category, ThreatCategory::Internal);
    }

    #[test]
    fn test_vlm_threats() {
        let threats = threat_templates::vlm_experiment_threats();
        assert!(!threats.is_empty());
        assert!(threats.iter().any(|t| t.name == "Model Hallucination"));
    }

    #[test]
    fn test_report_generation() {
        let mut analyzer = ThreatAnalyzer::new("VLM Experiment");

        for threat in threat_templates::vlm_experiment_threats() {
            analyzer.add_threat(threat);
        }

        let report = analyzer.generate_report();
        let markdown = report.to_markdown();

        assert!(markdown.contains("Validity Threats Analysis"));
        assert!(markdown.contains("Internal Validity"));
    }
}
