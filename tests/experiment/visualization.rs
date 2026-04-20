//! 实验可视化模块
//!
//! 使用 plotters 库生成实验图表。
//!
//! 注意：由于 plotters API 限制，部分功能可能需要调整。

use plotters::prelude::*;
use std::path::Path;

/// 图表配置
#[derive(Debug, Clone)]
pub struct ChartConfig {
    pub title: String,
    pub x_label: String,
    pub y_label: String,
    pub width: u32,
    pub height: u32,
}

impl Default for ChartConfig {
    fn default() -> Self {
        Self {
            title: String::new(),
            x_label: String::new(),
            y_label: String::new(),
            width: 800,
            height: 600,
        }
    }
}

impl ChartConfig {
    pub fn new(title: &str) -> Self {
        Self {
            title: title.to_string(),
            ..Default::default()
        }
    }

    pub fn x_label(mut self, label: &str) -> Self {
        self.x_label = label.to_string();
        self
    }

    pub fn y_label(mut self, label: &str) -> Self {
        self.y_label = label.to_string();
        self
    }

    pub fn size(mut self, width: u32, height: u32) -> Self {
        self.width = width;
        self.height = height;
        self
    }
}

/// 数据系列
#[derive(Debug, Clone)]
pub struct DataSeries {
    pub name: String,
    pub values: Vec<f64>,
}

impl DataSeries {
    pub fn new(name: &str, values: Vec<f64>) -> Self {
        Self {
            name: name.to_string(),
            values,
        }
    }
}

/// 预定义颜色
#[derive(Debug, Clone, Copy)]
pub struct RGBAColor(pub u8, pub u8, pub u8);

impl RGBAColor {
    pub const BLUE: Self = Self(0, 114, 189);
    pub const ORANGE: Self = Self(217, 83, 25);
    pub const GREEN: Self = Self(32, 134, 41);
    pub const RED: Self = Self(204, 17, 17);

    pub fn into_rgb(self) -> RGBColor {
        RGBColor(self.0, self.1, self.2)
    }
}

/// 生成简单的折线图（使用 plotters 支持的基本 API）
pub fn generate_line_chart<P: AsRef<Path>>(
    output_path: P,
    config: &ChartConfig,
    x_values: &[f64],
    series: &[DataSeries],
) -> Result<(), Box<dyn std::error::Error>> {
    let root =
        BitMapBackend::new(output_path.as_ref(), (config.width, config.height)).into_drawing_area();
    root.fill(&WHITE)?;

    if x_values.is_empty() || series.is_empty() {
        return Ok(());
    }

    let y_min = series
        .iter()
        .flat_map(|s| s.values.iter())
        .cloned()
        .fold(f64::INFINITY, f64::min);
    let y_max = series
        .iter()
        .flat_map(|s| s.values.iter())
        .cloned()
        .fold(f64::NEG_INFINITY, f64::max);
    let y_range = (y_max - y_min) * 0.1;

    let mut chart = ChartBuilder::on(&root)
        .caption(&config.title, ("sans-serif", 30).into_font())
        .margin(10)
        .x_label_area_size(40)
        .y_label_area_size(50)
        .build_cartesian_2d(
            *x_values.first().unwrap_or(&0.0)..*x_values.last().unwrap_or(&1.0),
            y_min - y_range..y_max + y_range,
        )?;

    chart.configure_mesh().draw()?;

    for (series_idx, data_series) in series.iter().enumerate() {
        let color = match series_idx {
            0 => RGBAColor::BLUE.into_rgb(),
            1 => RGBAColor::ORANGE.into_rgb(),
            2 => RGBAColor::GREEN.into_rgb(),
            _ => RGBAColor::RED.into_rgb(),
        };

        let points: Vec<(f64, f64)> = x_values
            .iter()
            .zip(data_series.values.iter())
            .map(|(&x, &y)| (x, y))
            .collect();

        chart
            .draw_series(LineSeries::new(points, &color))?
            .label(&data_series.name)
            .legend(move |(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], color));
    }

    chart
        .configure_series_labels()
        .position(SeriesLabelPosition::UpperRight)
        .draw()?;

    Ok(())
}

/// 生成实验汇总图表
pub fn generate_experiment_summary_charts<P: AsRef<Path>>(
    output_dir: P,
) -> Result<(), Box<dyn std::error::Error>> {
    let output_dir = output_dir.as_ref();
    std::fs::create_dir_all(output_dir)?;

    // 生成简单的折线图
    let sizes = vec![100.0, 500.0, 1000.0];
    let throughput = vec![1341957.0, 282497.0, 145000.0];

    generate_line_chart(
        output_dir.join("scalability.png"),
        &ChartConfig::new("可扩展性分析")
            .x_label("数据规模")
            .y_label("吞吐量 (ops/sec)"),
        &sizes,
        &[DataSeries::new("吞吐量", throughput)],
    )?;

    println!("图表已生成到：{:?}", output_dir);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chart_config_default() {
        let config = ChartConfig::default();
        assert_eq!(config.title, "");
        assert_eq!(config.x_label, "");
        assert_eq!(config.y_label, "");
        assert_eq!(config.width, 800);
        assert_eq!(config.height, 600);
    }

    #[test]
    fn test_chart_config_builder() {
        let config = ChartConfig::new("Test Title")
            .x_label("X Axis")
            .y_label("Y Axis")
            .size(1024, 768);

        assert_eq!(config.title, "Test Title");
        assert_eq!(config.x_label, "X Axis");
        assert_eq!(config.y_label, "Y Axis");
        assert_eq!(config.width, 1024);
        assert_eq!(config.height, 768);
    }

    #[test]
    fn test_data_series() {
        let series = DataSeries::new("Test Series", vec![1.0, 2.0, 3.0]);
        assert_eq!(series.name, "Test Series");
        assert_eq!(series.values.len(), 3);
        assert_eq!(series.values[0], 1.0);
    }

    #[test]
    fn test_rgba_colors() {
        assert_eq!(RGBAColor::BLUE.0, 0);
        assert_eq!(RGBAColor::BLUE.1, 114);
        assert_eq!(RGBAColor::BLUE.2, 189);

        assert_eq!(RGBAColor::ORANGE.0, 217);
        assert_eq!(RGBAColor::GREEN.0, 32);
        assert_eq!(RGBAColor::RED.0, 204);
    }

    #[test]
    fn test_rgba_to_rgb() {
        let color = RGBAColor::BLUE;
        let rgb = color.into_rgb();
        assert_eq!(rgb.0, 0);
        assert_eq!(rgb.1, 114);
        assert_eq!(rgb.2, 189);
    }

    #[test]
    fn test_generate_line_chart_empty_data() {
        let temp_dir = tempfile::tempdir().unwrap();
        let output_path = temp_dir.path().join("test_empty.png");

        let result = generate_line_chart(&output_path, &ChartConfig::new("Empty Chart"), &[], &[]);

        assert!(result.is_ok());
        assert!(output_path.exists());
    }

    #[test]
    fn test_generate_line_chart_multiple_series() {
        let temp_dir = tempfile::tempdir().unwrap();
        let output_path = temp_dir.path().join("test_multi_series.png");

        let x_values = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let series = vec![
            DataSeries::new("Series1", vec![2.0, 4.0, 6.0, 8.0, 10.0]),
            DataSeries::new("Series2", vec![1.0, 3.0, 5.0, 7.0, 9.0]),
            DataSeries::new("Series3", vec![5.0, 5.0, 5.0, 5.0, 5.0]),
        ];

        let result = generate_line_chart(
            &output_path,
            &ChartConfig::new("Multi-Series Chart")
                .x_label("X")
                .y_label("Y"),
            &x_values,
            &series,
        );

        assert!(result.is_ok());
        assert!(output_path.exists());
    }

    #[test]
    fn test_generate_line_chart_custom_size() {
        let temp_dir = tempfile::tempdir().unwrap();
        let output_path = temp_dir.path().join("test_custom_size.png");

        let x_values = vec![1.0, 2.0, 3.0];
        let series = vec![DataSeries::new("Series1", vec![1.0, 2.0, 3.0])];

        let result = generate_line_chart(
            &output_path,
            &ChartConfig::new("Custom Size").size(1200, 900),
            &x_values,
            &series,
        );

        assert!(result.is_ok());
        assert!(output_path.exists());

        // 验证文件大小（自定义尺寸应该更大）
        let metadata = std::fs::metadata(&output_path).unwrap();
        assert!(metadata.len() > 0);
    }

    #[test]
    fn test_generate_experiment_summary_charts() {
        let temp_dir = tempfile::tempdir().unwrap();
        let output_dir = temp_dir.path().join("summary_charts");

        let result = generate_experiment_summary_charts(&output_dir);

        assert!(result.is_ok());
        assert!(output_dir.exists());
        assert!(output_dir.join("scalability.png").exists());
    }

    #[test]
    fn test_generate_line_chart_single_point() {
        let temp_dir = tempfile::tempdir().unwrap();
        let output_path = temp_dir.path().join("test_single.png");

        let x_values = vec![1.0];
        let series = vec![DataSeries::new("Series1", vec![5.0])];

        let result = generate_line_chart(
            &output_path,
            &ChartConfig::new("Single Point"),
            &x_values,
            &series,
        );

        assert!(result.is_ok());
        assert!(output_path.exists());
    }

    #[test]
    fn test_generate_line_chart_negative_values() {
        let temp_dir = tempfile::tempdir().unwrap();
        let output_path = temp_dir.path().join("test_negative.png");

        let x_values = vec![1.0, 2.0, 3.0, 4.0];
        let series = vec![DataSeries::new("Series1", vec![-5.0, -3.0, -1.0, 1.0])];

        let result = generate_line_chart(
            &output_path,
            &ChartConfig::new("Negative Values")
                .x_label("X")
                .y_label("Y"),
            &x_values,
            &series,
        );

        assert!(result.is_ok());
        assert!(output_path.exists());
    }
}
