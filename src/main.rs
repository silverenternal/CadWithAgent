//! CadAgent CLI
//!
//! 命令行接口

use clap::{Parser, Subcommand};
use cadagent::prelude::*;
use cadagent::tools::ToolRegistry;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "cadagent")]
#[command(about = "CAD 几何处理工具链 - 基于 tokitai 的 AI 驱动管线")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// 启用详细输出
    #[arg(short, long, global = true)]
    verbose: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// 解析 SVG 文件
    ParseSvg {
        /// SVG 文件路径
        #[arg(short, long)]
        input: PathBuf,

        /// 输出 JSON 路径
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// 测量图形
    Measure {
        /// 测量类型
        #[arg(short, long)]
        kind: MeasureKind,

        /// 输入数据（JSON 格式）
        #[arg(short, long)]
        data: String,
    },

    /// 检测房间
    DetectRooms {
        /// 输入文件路径
        #[arg(short, long)]
        input: PathBuf,
    },

    /// 导出 DXF
    ExportDxf {
        /// 输入 JSON 文件路径
        #[arg(short, long)]
        input: PathBuf,

        /// 输出 DXF 路径
        #[arg(short, long)]
        output: PathBuf,
    },

    /// 生成 Geo-CoT 数据
    GenerateCot {
        /// 输入文件路径
        #[arg(short, long)]
        input: PathBuf,

        /// 任务描述
        #[arg(short, long)]
        task: String,
    },

    /// 生成 QA 数据
    GenerateQa {
        /// 输入文件路径
        #[arg(short, long)]
        input: PathBuf,

        /// 问题类型
        #[arg(short, long)]
        kind: Option<String>,
    },

    /// 一致性检查
    CheckConsistency {
        /// 输入文件路径
        #[arg(short, long)]
        input: PathBuf,
    },

    /// 列出所有可用工具
    ListTools,
}

#[derive(clap::ValueEnum, Clone, Debug)]
enum MeasureKind {
    Length,
    Area,
    Angle,
}

fn main() -> anyhow::Result<()> {
    // 初始化日志
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();
    let registry = ToolRegistry::new();

    match cli.command {
        Commands::ParseSvg { input, output } => {
            let result = SvgParser::parse(&input)?;
            
            let json = serde_json::to_string_pretty(&result)?;
            
            match output {
                Some(path) => {
                    std::fs::write(&path, &json)?;
                    println!("已保存到：{}", path.display());
                }
                None => println!("{}", json),
            }
        }

        Commands::Measure { kind, data } => {
            let args: serde_json::Value = serde_json::from_str(&data)?;
            
            let tool_name = match kind {
                MeasureKind::Length => "measure_length",
                MeasureKind::Area => "measure_area",
                MeasureKind::Angle => "measure_angle",
            };

            let result = registry.call(tool_name, args)?;
            println!("{}", serde_json::to_string_pretty(&result)?);
        }

        Commands::DetectRooms { input } => {
            // 读取输入文件
            let content = std::fs::read_to_string(&input)?;
            let primitives: Vec<Primitive> = serde_json::from_str(&content)?;

            use cadagent::topology::room_detect::detect_rooms;
            let result = detect_rooms(&primitives);

            println!("{}", serde_json::to_string_pretty(&result)?);
        }

        Commands::ExportDxf { input, output } => {
            let content = std::fs::read_to_string(&input)?;
            let primitives: Vec<Primitive> = serde_json::from_str(&content)?;

            let result = DxfExporter::export(&primitives, &output)?;
            println!("导出成功：{} ({} 个图元)", result.path, result.entity_count);
        }

        Commands::GenerateCot { input, task } => {
            let content = std::fs::read_to_string(&input)?;
            let primitives: Vec<Primitive> = serde_json::from_str(&content)?;

            let generator = GeoCotGenerator::new();
            let cot_data = generator.generate(&primitives, &task);

            println!("{}", serde_json::to_string_pretty(&cot_data)?);
        }

        Commands::GenerateQa { input, kind } => {
            let content = std::fs::read_to_string(&input)?;
            let primitives: Vec<Primitive> = serde_json::from_str(&content)?;

            use cadagent::cot::qa::QaGenerator;
            let generator = QaGenerator::new();
            
            let qa_pairs = match kind {
                Some(k) => generator.generate_all(&primitives)
                    .into_iter()
                    .filter(|qa| qa.question_type == k)
                    .collect::<Vec<_>>(),
                None => generator.generate_all(&primitives),
            };

            println!("{}", serde_json::to_string_pretty(&qa_pairs)?);
        }

        Commands::CheckConsistency { input } => {
            let content = std::fs::read_to_string(&input)?;
            let primitives: Vec<Primitive> = serde_json::from_str(&content)?;

            use cadagent::metrics::consistency::ConsistencyChecker;
            let checker = ConsistencyChecker::new();
            let result = checker.check_all(&primitives);

            println!("一致性得分：{:.2}", result.score);
            println!("检查结果：{}", if result.passed { "通过" } else { "失败" });
            
            if !result.errors.is_empty() {
                println!("\n错误:");
                for error in &result.errors {
                    println!("  - {}", error);
                }
            }
        }

        Commands::ListTools => {
            let tools = registry.list_tools();
            println!("可用工具 ({} 个):", tools.len());
            println!();
            
            for tool in tools {
                println!("  {} - {}", tool.name, tool.description);
            }
        }
    }

    Ok(())
}
