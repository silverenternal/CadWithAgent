#!/usr/bin/env python3
"""
性能回归检测脚本

用于检测实验结果中的性能回归，当性能下降超过阈值时失败。

使用方法:
    python check_regression.py [--threshold THRESHOLD] [--baseline BASELINE]

参数:
    --threshold: 性能下降阈值（百分比），默认 5.0
    --baseline: 基准结果文件路径，默认使用最新的基准数据
"""

import json
import sys
import argparse
from pathlib import Path
from typing import Dict, List, Optional, Tuple


# 性能指标配置
PERFORMANCE_METRICS = {
    "throughput": {"direction": "higher", "min_value": 1000.0},  # 吞吐量越高越好
    "latency": {"direction": "lower", "max_value": 100.0},  # 延迟越低越好
    "accuracy": {"direction": "higher", "min_value": 0.9},  # 准确率越高越好
}

# 默认阈值（性能下降百分比）
DEFAULT_THRESHOLD = 5.0


def load_json_file(path: Path) -> Dict:
    """加载 JSON 文件"""
    with open(path, 'r', encoding='utf-8') as f:
        return json.load(f)


def get_latest_result_file(results_dir: Path, pattern: str = "*_result.json") -> Optional[Path]:
    """获取最新的结果文件"""
    files = list(results_dir.glob(pattern))
    if not files:
        return None
    
    # 按修改时间排序
    files.sort(key=lambda f: f.stat().st_mtime, reverse=True)
    return files[0]


def extract_metrics(result: Dict) -> Dict[str, float]:
    """从实验结果中提取性能指标"""
    metrics = result.get("metrics", {})
    
    # 标准化指标名称
    normalized = {}
    for key, value in metrics.items():
        normalized_key = key.lower()
        if "throughput" in normalized_key:
            normalized["throughput"] = value
        elif "latency" in normalized_key or "p50" in normalized_key or "p95" in normalized_key:
            normalized["latency"] = value
        elif "accuracy" in normalized_key:
            normalized["accuracy"] = value
        else:
            # 其他数值指标也保留
            normalized[normalized_key] = value
    
    return normalized


def compare_metrics(
    current: Dict[str, float],
    baseline: Dict[str, float],
    threshold: float
) -> Tuple[bool, List[str]]:
    """
    比较当前指标与基准指标
    
    返回:
        (passed, messages): 是否通过检测，消息列表
    """
    messages = []
    passed = True
    
    for metric_name, config in PERFORMANCE_METRICS.items():
        if metric_name not in current:
            continue
        
        if metric_name not in baseline:
            messages.append(f"⚠️  基准数据缺少指标：{metric_name}")
            continue
        
        current_value = current[metric_name]
        baseline_value = baseline[metric_name]
        
        # 计算变化百分比
        if baseline_value != 0:
            change_pct = ((current_value - baseline_value) / abs(baseline_value)) * 100
        else:
            change_pct = 0.0 if current_value == 0 else float('inf')
        
        # 根据指标方向判断是否回归
        is_regression = False
        if config["direction"] == "higher":
            # 吞吐量、准确率：下降为回归
            if change_pct < -threshold:
                is_regression = True
        else:
            # 延迟：上升为回归
            if change_pct > threshold:
                is_regression = True
        
        # 生成消息
        direction_symbol = "↑" if change_pct > 0 else "↓"
        status = "❌ 回归" if is_regression else "✓"
        
        msg = (
            f"{status} {metric_name}: "
            f"{baseline_value:.2f} → {current_value:.2f} "
            f"({direction_symbol} {abs(change_pct):.2f}%)"
        )
        messages.append(msg)
        
        if is_regression:
            passed = False
    
    # 检查额外指标
    for metric_name in current:
        if metric_name in PERFORMANCE_METRICS:
            continue
        
        if metric_name not in baseline:
            messages.append(f"ℹ️  新增指标：{metric_name} = {current[metric_name]:.4f}")
            continue
        
        current_value = current[metric_name]
        baseline_value = baseline[metric_name]
        
        if baseline_value != 0:
            change_pct = ((current_value - baseline_value) / abs(baseline_value)) * 100
        else:
            change_pct = 0.0 if current_value == 0 else float('inf')
        
        # 默认：超过 10% 变化视为潜在问题
        if abs(change_pct) > 10:
            direction_symbol = "↑" if change_pct > 0 else "↓"
            messages.append(f"⚠️  {metric_name}: {direction_symbol} {abs(change_pct):.2f}%")
    
    return passed, messages


def check_absolute_thresholds(metrics: Dict[str, float]) -> Tuple[bool, List[str]]:
    """检查绝对阈值"""
    messages = []
    passed = True
    
    for metric_name, config in PERFORMANCE_METRICS.items():
        if metric_name not in metrics:
            continue
        
        value = metrics[metric_name]
        
        if "min_value" in config and value < config["min_value"]:
            messages.append(f"❌ {metric_name} ({value:.4f}) 低于最小阈值 ({config['min_value']})")
            passed = False
        
        if "max_value" in config and value > config["max_value"]:
            messages.append(f"❌ {metric_name} ({value:.4f}) 高于最大阈值 ({config['max_value']})")
            passed = False
    
    return passed, messages


def main():
    parser = argparse.ArgumentParser(description="性能回归检测")
    parser.add_argument(
        "--threshold",
        type=float,
        default=DEFAULT_THRESHOLD,
        help=f"性能下降阈值（百分比），默认 {DEFAULT_THRESHOLD}"
    )
    parser.add_argument(
        "--baseline",
        type=Path,
        default=None,
        help="基准结果文件路径"
    )
    parser.add_argument(
        "--current",
        type=Path,
        default=None,
        help="当前结果文件路径"
    )
    parser.add_argument(
        "--results-dir",
        type=Path,
        default=Path("tests/experiment/results"),
        help="结果目录路径"
    )
    parser.add_argument(
        "--output",
        type=Path,
        default=None,
        help="输出报告路径"
    )
    
    args = parser.parse_args()
    
    results_dir = args.results_dir
    
    # 加载当前结果
    if args.current:
        current_path = args.current
    else:
        current_path = get_latest_result_file(results_dir, "performance_benchmark_result.json")
    
    if not current_path or not current_path.exists():
        print(f"❌ 错误：找不到当前结果文件：{current_path}")
        sys.exit(1)
    
    print(f"📊 加载当前结果：{current_path}")
    current_data = load_json_file(current_path)
    current_metrics = extract_metrics(current_data)
    
    # 加载基准结果
    if args.baseline:
        baseline_path = args.baseline
    else:
        # 使用上一次的结果作为基准
        baseline_path = results_dir / "baseline" / "performance_benchmark_result.json"
        if not baseline_path.exists():
            # 如果没有基准，复制当前结果作为初始基准
            baseline_path.parent.mkdir(parents=True, exist_ok=True)
            with open(baseline_path, 'w', encoding='utf-8') as f:
                json.dump(current_data, f, indent=2, ensure_ascii=False)
            print(f"ℹ️  创建初始基准：{baseline_path}")
    
    if baseline_path and baseline_path.exists():
        print(f"📊 加载基准结果：{baseline_path}")
        baseline_data = load_json_file(baseline_path)
        baseline_metrics = extract_metrics(baseline_data)
    else:
        print("⚠️  警告：找不到基准结果，跳过回归比较")
        baseline_metrics = {}
    
    # 检查绝对阈值
    print("\n=== 绝对阈值检查 ===")
    threshold_passed, threshold_messages = check_absolute_thresholds(current_metrics)
    for msg in threshold_messages:
        print(msg)
    
    # 回归比较
    print("\n=== 性能回归比较 ===")
    if baseline_metrics:
        regression_passed, regression_messages = compare_metrics(
            current_metrics, baseline_metrics, args.threshold
        )
        for msg in regression_messages:
            print(msg)
    else:
        regression_passed = True
        regression_messages = ["无基准数据，跳过比较"]
    
    # 总体结果
    overall_passed = threshold_passed and regression_passed
    
    print("\n" + "=" * 50)
    if overall_passed:
        print("✅ 性能检测通过")
        exit_code = 0
    else:
        print("❌ 性能检测失败：检测到性能回归")
        exit_code = 1
    
    # 生成报告
    if args.output:
        report = {
            "passed": overall_passed,
            "threshold": args.threshold,
            "current_file": str(current_path),
            "baseline_file": str(baseline_path) if baseline_path.exists() else None,
            "current_metrics": current_metrics,
            "baseline_metrics": baseline_metrics,
            "messages": threshold_messages + regression_messages,
        }
        
        args.output.parent.mkdir(parents=True, exist_ok=True)
        with open(args.output, 'w', encoding='utf-8') as f:
            json.dump(report, f, indent=2, ensure_ascii=False)
        print(f"\n📄 报告已保存：{args.output}")
    
    sys.exit(exit_code)


if __name__ == "__main__":
    main()
