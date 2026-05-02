use chrono::{Datelike, Local, NaiveDateTime};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::warn;

// ─────────────────────────────────────────────
// 常量：6个基础桶 + 3个聚合桶名称
// ─────────────────────────────────────────────

pub const BUCKET_BASE: &[&str] = &[
    "common_limited",
    "common_unlimited",
    "regional_limited",
    "regional_unlimited",
    "targeted_limited",
    "targeted_unlimited",
];

pub const BUCKET_AGGREGATE: &[&str] = &["所有通用", "所有免流", "所有流量"];

/// 9 桶中文名映射
pub fn bucket_display_name(key: &str) -> &str {
    match key {
        "common_limited" => "通用有限",
        "common_unlimited" => "通用不限",
        "regional_limited" => "区域有限",
        "regional_unlimited" => "区域不限",
        "targeted_limited" => "免流有限",
        "targeted_unlimited" => "免流不限",
        "所有通用" => "所有通用",
        "所有免流" => "所有免流",
        "所有流量" => "所有流量",
        _ => key,
    }
}

// ─────────────────────────────────────────────
// 数据结构
// ─────────────────────────────────────────────

/// 单个流量桶的汇总值（单位 MB）
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FlowBucket {
    pub total: f64,
    pub used: f64,
    pub remain: f64,
}

/// 差值统计
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FlowDiff {
    /// 本次用量（相对于上次查询的差值）
    pub uused: f64,
    /// 今日累计用量
    pub today: f64,
}

/// 流量包详情条目（用于前端展示）
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PackageDetail {
    pub label: String,
    pub value: String,
}

/// 单个流量包
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct FlowPackage {
    /// feePolicyId
    pub tag: String,
    /// feePolicyName
    pub name: String,
    /// "1" | "2" | "3"
    pub flow_type: String,
    /// "01" | "13" | "I3" | "02" | "03"
    pub resource_type: String,
    /// 总量 MB
    pub total: f64,
    /// 已用 MB
    #[serde(rename = "use")]
    pub used: f64,
    /// 剩余 MB
    pub remain: f64,
    /// 到期日期（空串 = 长期有效）
    pub end_date: String,
    /// 是否公免
    #[serde(default)]
    pub is_public_free: bool,
    /// 类型标签：通用 / 区域 / 定向 / 公免
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#type: Option<String>,
    /// 副卡列表原始数据（共享流量场景）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vice_card_list: Option<Vec<ViceCard>>,
    /// 前端展示详情
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<Vec<PackageDetail>>,
}

/// 副卡信息
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ViceCard {
    pub usernumber: String,
    #[serde(default)]
    pub r#use: f64,
    #[serde(default)]
    pub current_login_flag: String,
    #[serde(default)]
    pub vice_card_flag: String,
}

/// 余额信息
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BalanceInfo {
    pub balance: String,
    pub real_fee: String,
    pub can_use_fee: String,
}

/// 分析结果
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FlowAnalysis {
    /// 查询时间戳
    pub timestamp: String,
    /// 主套餐名称
    pub main_package: String,
    /// 解析后的所有流量包
    pub packages: Vec<FlowPackage>,
    /// 9 桶数据（6 基础 + 3 聚合）
    pub buckets: HashMap<String, FlowBucket>,
    /// 差值统计
    pub diff: HashMap<String, FlowDiff>,
    /// 距上次查询的时间间隔（可读字符串）
    #[serde(skip_serializing_if = "String::is_empty")]
    pub time_interval: String,
    /// 余额信息
    #[serde(skip_serializing_if = "Option::is_none")]
    pub balance: Option<BalanceInfo>,
}

// ─────────────────────────────────────────────
// 包类型判断
// ─────────────────────────────────────────────

/// 判断流量包属于 common / regional / targeted
fn determine_package_type(
    flow_type: &str,
    resource_type: &str,
    is_public_free: bool,
) -> &'static str {
    // 通用流量：flowType=1 且 resourceType=01 或 1
    if flow_type == "1" && (resource_type == "01" || resource_type == "1") {
        return "common";
    }
    // 区域流量：flowType=2/3 且 resourceType=01 或 1
    if (flow_type == "2" || flow_type == "3") && (resource_type == "01" || resource_type == "1") {
        return "regional";
    }
    // 定向免流：flowType=2/3 且 resourceType=13 或 I3
    if (flow_type == "2" || flow_type == "3") && (resource_type == "13" || resource_type == "I3") {
        return "targeted";
    }
    // 公免标记 或 resourceType=13/I3
    if is_public_free || resource_type == "13" || resource_type == "I3" {
        return "targeted";
    }
    // 默认归类为通用
    "common"
}

/// 获取流量包类型标签（中文）
fn get_package_type_label(
    flow_type: &str,
    resource_type: &str,
    is_public_free: bool,
) -> &'static str {
    if is_public_free {
        return "公免";
    }
    match determine_package_type(flow_type, resource_type, is_public_free) {
        "common" => "通用",
        "regional" => "区域",
        "targeted" => "定向",
        _ => "通用",
    }
}

// ─────────────────────────────────────────────
// 格式化工具
// ─────────────────────────────────────────────

/// 格式化流量值（MB → 自动选单位）
pub fn format_flow(value: f64) -> String {
    if !value.is_finite() || value < 0.0 {
        return "0".to_string();
    }

    let units = ["M", "G", "T", "P"];
    let mut num = value;
    let mut idx = 0;

    while num >= 1024.0 && idx < units.len() - 1 {
        num /= 1024.0;
        idx += 1;
    }

    // 保留两位小数，去除尾部零
    let s = format!("{:.2}", num);
    let s = s.trim_end_matches('0').trim_end_matches('.');
    format!("{}{}", s, units[idx])
}

/// 格式化秒数为可读时间间隔，如 "2小时35分钟"
pub fn format_interval(seconds: i64) -> String {
    if seconds <= 0 {
        return String::new();
    }
    let hours = seconds / 3600;
    let minutes = (seconds % 3600) / 60;
    let secs = seconds % 60;

    let mut parts = Vec::new();
    if hours > 0 {
        parts.push(format!("{}小时", hours));
    }
    if minutes > 0 {
        parts.push(format!("{}分钟", minutes));
    }
    if secs > 0 && hours == 0 {
        parts.push(format!("{}秒", secs));
    }
    if parts.is_empty() {
        return "0秒".to_string();
    }
    parts.join("")
}

// ─────────────────────────────────────────────
// 解析联通 API 返回的 JSON
// ─────────────────────────────────────────────

/// 从联通 API 返回的 JSON 值中解析单条 detail 为 FlowPackage
fn parse_detail(p: &serde_json::Value) -> Option<FlowPackage> {
    let flow_type = p
        .get("flowType")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let resource_type = p
        .get("resourceType")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    if flow_type.is_empty() || resource_type.is_empty() {
        return None;
    }

    // 只处理流量类型（1/2/3），跳过语音(02)和短信(03)
    // 但 MlResources 场景下 resourceType 可能会被覆盖为 "13"，这里仍然保留原始值
    // 在 process_packages 层面做跳过处理

    Some(FlowPackage {
        tag: p
            .get("feePolicyId")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        name: p
            .get("feePolicyName")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        flow_type,
        resource_type,
        total: parse_f64_str(p.get("total")),
        used: parse_f64_str(p.get("use")),
        remain: parse_f64_str(p.get("remain")),
        end_date: p
            .get("endDate")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        is_public_free: false,
        r#type: None,
        vice_card_list: None,
        details: None,
    })
}

/// 解析可能是字符串 "10240.00" 或数字 10240.0 的值
fn parse_f64_str(v: Option<&serde_json::Value>) -> f64 {
    match v {
        Some(serde_json::Value::Number(n)) => n.as_f64().unwrap_or(0.0),
        Some(serde_json::Value::String(s)) => s.parse::<f64>().unwrap_or(0.0),
        _ => 0.0,
    }
}

/// 从副卡列表中查找当前手机号的使用量
fn get_current_mobile_usage(vice_card_list: &[ViceCard], mobile: &str) -> f64 {
    for card in vice_card_list {
        if is_current_mobile(&card.usernumber, mobile) {
            return card.r#use;
        }
    }
    0.0
}

/// 判断号码是否匹配当前手机号（支持隐藏格式 138****5678）
fn is_current_mobile(phone_number: &str, target_mobile: &str) -> bool {
    if phone_number.is_empty() || target_mobile.is_empty() {
        return false;
    }
    // 处理隐藏格式 138****5678
    if phone_number.len() == 11 && phone_number.contains("****") {
        let prefix = &phone_number[..3];
        let suffix = &phone_number[7..];
        return target_mobile.starts_with(prefix) && target_mobile.ends_with(suffix);
    }
    phone_number == target_mobile
}

/// 解析副卡列表 JSON
fn parse_vice_cards(v: &serde_json::Value) -> Option<Vec<ViceCard>> {
    let arr = v.as_array()?;
    let cards: Vec<ViceCard> = arr
        .iter()
        .filter_map(|item| {
            Some(ViceCard {
                usernumber: item
                    .get("usernumber")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                r#use: parse_f64_str(item.get("use")),
                current_login_flag: item
                    .get("currentLoginFlag")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                vice_card_flag: item
                    .get("viceCardflag")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
            })
        })
        .collect();
    if cards.is_empty() {
        None
    } else {
        Some(cards)
    }
}

/// 构建流量包详情（用于前端展示）
fn build_package_details(pkg: &FlowPackage) -> Vec<PackageDetail> {
    let mut details = Vec::new();

    // 有效期
    if !pkg.end_date.is_empty() {
        details.push(PackageDetail {
            label: "有效期".to_string(),
            value: pkg.end_date.clone(),
        });
    }

    // 副卡列表
    if let Some(cards) = &pkg.vice_card_list {
        for card in cards {
            let is_current = card.current_login_flag == "1";
            let is_vice = card.vice_card_flag == "1";

            let label = if is_current {
                format!("{} (当前)", card.usernumber)
            } else if is_vice {
                format!("{} (主卡)", card.usernumber)
            } else {
                format!("{} (副卡)", card.usernumber)
            };

            details.push(PackageDetail {
                label,
                value: format_flow(card.r#use),
            });
        }
    }

    details
}

/// 处理联通 API 返回的所有流量包（资源）
///
/// 对应 PHP 的 processPackages 逻辑：
/// 1. shareData.details → 共享流量（提取当前手机号用量）
/// 2. unshared / resources → 独享 / 普通资源
/// 3. MlResources → 公免流量（强制标记为定向无限）
fn process_packages(data: &serde_json::Value, mobile: &str) -> Vec<FlowPackage> {
    let mut packages = Vec::new();

    let has_share = data
        .get("shareData")
        .and_then(|v| v.as_object())
        .map(|o| !o.is_empty())
        .unwrap_or(false);

    // A. 共享流量
    if has_share {
        if let Some(details) = data
            .get("shareData")
            .and_then(|v| v.get("details"))
            .and_then(|v| v.as_array())
        {
            for p in details {
                if let Some(mut pkg) = parse_detail(p) {
                    // 跳过非流量类型
                    if !is_flow_resource(&pkg.flow_type, &pkg.resource_type) {
                        continue;
                    }
                    // 从副卡列表提取当前手机号用量
                    if let Some(cards) =
                        parse_vice_cards(p.get("viceCardlist").unwrap_or(&serde_json::Value::Null))
                    {
                        pkg.used = get_current_mobile_usage(&cards, mobile);
                        pkg.vice_card_list = Some(cards);
                    }
                    packages.push(pkg);
                }
            }
        }
    }

    // B. 独享 / 普通资源
    let resource_key = if has_share { "unshared" } else { "resources" };
    if let Some(resources) = data.get(resource_key).and_then(|v| v.as_array()) {
        for resource in resources {
            if let Some(details) = resource.get("details").and_then(|v| v.as_array()) {
                for p in details {
                    if let Some(pkg) = parse_detail(p) {
                        if !is_flow_resource(&pkg.flow_type, &pkg.resource_type) {
                            continue;
                        }
                        packages.push(pkg);
                    }
                }
            }
        }
    }

    // C. 公免流量（MlResources）
    if let Some(ml_resources) = data.get("MlResources").and_then(|v| v.as_array()) {
        for resource in ml_resources {
            if let Some(details) = resource.get("details").and_then(|v| v.as_array()) {
                for p in details {
                    let flow_type = p.get("flowType").and_then(|v| v.as_str()).unwrap_or("");
                    if flow_type.is_empty() {
                        continue;
                    }
                    if let Some(mut pkg) = parse_detail(p) {
                        pkg.resource_type = "13".to_string(); // 强制标记为定向
                        pkg.total = 0.0; // 无限流量
                        pkg.remain = 0.0; // 无限流量
                        pkg.is_public_free = true;
                        packages.push(pkg);
                    }
                }
            }
        }
    }

    packages
}

/// 判断是否为流量资源（跳过语音 02 和短信 03）
fn is_flow_resource(flow_type: &str, resource_type: &str) -> bool {
    // flowType 必须是 1/2/3
    if flow_type != "1" && flow_type != "2" && flow_type != "3" {
        return false;
    }
    // 跳过语音(02)和短信(03)
    if resource_type == "02" || resource_type == "03" {
        return false;
    }
    true
}

// ─────────────────────────────────────────────
// 核心分析函数
// ─────────────────────────────────────────────

/// 从联通 API 原始 JSON 解析并执行 9 桶分析
///
/// 输入 `data` 是联通 API 返回的完整 JSON（已解析为 Value）。
/// `mobile` 用于在共享流量中提取当前用户的用量。
pub fn analyze_flow(data: &serde_json::Value, mobile: &str) -> FlowAnalysis {
    let main_package = data
        .get("packageName")
        .and_then(|v| v.as_str())
        .unwrap_or("未知套餐")
        .to_string();

    // 初始化 6 个基础桶
    let mut buckets: HashMap<String, FlowBucket> = HashMap::new();
    for key in BUCKET_BASE {
        buckets.insert(key.to_string(), FlowBucket::default());
    }

    // 解析所有流量包
    let mut packages = process_packages(data, mobile);

    // 分类统计到 6 个基础桶
    for pkg in &packages {
        let bucket_type =
            determine_package_type(&pkg.flow_type, &pkg.resource_type, pkg.is_public_free);

        // 公免流量包（total=0, remain=0）不计入桶统计
        if pkg.total == 0.0 && pkg.remain == 0.0 && pkg.is_public_free {
            continue;
        }

        let suffix = if pkg.total > 0.0 {
            "limited"
        } else {
            "unlimited"
        };
        let key = format!("{}_{}", bucket_type, suffix);

        if let Some(bucket) = buckets.get_mut(&key) {
            bucket.total += pkg.total;
            bucket.used += pkg.used;
            bucket.remain += pkg.remain;
        }
    }

    // 为每个流量包添加详情和类型标签
    for pkg in &mut packages {
        pkg.details = Some(build_package_details(pkg));
        pkg.r#type = Some(
            get_package_type_label(&pkg.flow_type, &pkg.resource_type, pkg.is_public_free)
                .to_string(),
        );
    }

    // 生成 9 桶（包含聚合桶）
    let full_buckets = generate_full_buckets(&buckets);

    // 首次查询的 diff（无 previous 时）
    let diff = build_initial_diff(&full_buckets);

    FlowAnalysis {
        timestamp: Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
        main_package,
        packages,
        buckets: full_buckets,
        diff,
        time_interval: String::new(),
        balance: None,
    }
}

// ─────────────────────────────────────────────
// 聚合桶生成
// ─────────────────────────────────────────────

/// 计算聚合桶：累加指定 keys 的桶值
///
/// 特殊处理无限流量桶（total=0, remain<0）：只累加 used
fn calculate_aggregate(buckets: &HashMap<String, FlowBucket>, keys: &[&str]) -> FlowBucket {
    let mut result = FlowBucket::default();
    for key in keys {
        if let Some(bucket) = buckets.get(*key) {
            if bucket.total == 0.0 && bucket.remain < 0.0 {
                // 无限流量桶：只累加已用量
                result.used += bucket.used;
            } else {
                // 有限流量桶：正常累加
                result.total += bucket.total;
                result.used += bucket.used;
                result.remain += bucket.remain;
            }
        }
    }
    result
}

/// 从 6 个基础桶生成完整的 9 桶
pub fn generate_full_buckets(buckets: &HashMap<String, FlowBucket>) -> HashMap<String, FlowBucket> {
    let mut full = buckets.clone();

    let common_keys = [
        "common_limited",
        "common_unlimited",
        "regional_limited",
        "regional_unlimited",
    ];
    let targeted_keys = ["targeted_limited", "targeted_unlimited"];

    let all_common = calculate_aggregate(buckets, &common_keys);
    let all_targeted = calculate_aggregate(buckets, &targeted_keys);

    let all_flow = FlowBucket {
        total: all_common.total + all_targeted.total,
        used: all_common.used + all_targeted.used,
        remain: all_common.remain + all_targeted.remain,
    };

    full.insert("所有通用".to_string(), all_common);
    full.insert("所有免流".to_string(), all_targeted);
    full.insert("所有流量".to_string(), all_flow);

    full
}

// ─────────────────────────────────────────────
// 差值计算
// ─────────────────────────────────────────────

/// 生成首次查询的 diff（所有桶的 uused/today = current.used）
fn build_initial_diff(buckets: &HashMap<String, FlowBucket>) -> HashMap<String, FlowDiff> {
    let mut diff = HashMap::new();
    // 6 基础桶
    for key in BUCKET_BASE {
        let used = buckets.get(*key).map(|b| b.used).unwrap_or(0.0);
        diff.insert(
            key.to_string(),
            FlowDiff {
                uused: used,
                today: used,
            },
        );
    }
    // 3 聚合桶
    for key in BUCKET_AGGREGATE {
        let used = buckets.get(*key).map(|b| b.used).unwrap_or(0.0);
        diff.insert(
            key.to_string(),
            FlowDiff {
                uused: used,
                today: used,
            },
        );
    }
    diff
}

/// 计算流量差值统计
///
/// 对应 PHP 的 calculateDiff 逻辑：
///
/// - `current`  — 当前分析结果（含 buckets）
/// - `previous_json` — 上次查询保存的 FlowAnalysis JSON 字符串（可为空）
/// - `last_query_time` — 上次查询时间 "%Y-%m-%d %H:%M:%S"（可为空）
///
/// 返回完整 9 桶的 diff HashMap。
pub fn calculate_diff(
    current: &FlowAnalysis,
    previous_json: &str,
    last_query_time: &str,
) -> HashMap<String, FlowDiff> {
    // 首次运行或无历史数据
    if previous_json.is_empty() || last_query_time.is_empty() {
        return build_initial_diff(&current.buckets);
    }

    // 解析上次数据
    let previous: FlowAnalysis = match serde_json::from_str(previous_json) {
        Ok(p) => p,
        Err(e) => {
            warn!("解析上次查询数据失败: {}，使用首次逻辑", e);
            return build_initial_diff(&current.buckets);
        }
    };

    let now = Local::now();
    let today_str = now.format("%Y-%m-%d").to_string();

    let last_time = match NaiveDateTime::parse_from_str(last_query_time, "%Y-%m-%d %H:%M:%S") {
        Ok(t) => t,
        Err(e) => {
            warn!("解析上次查询时间失败: {}，使用首次逻辑", e);
            return build_initial_diff(&current.buckets);
        }
    };
    let last_date = last_time.format("%Y-%m-%d").to_string();

    // 检查是否跨月
    let current_month = now.year() * 12 + now.month() as i32;
    let previous_month = last_time.year() * 12 + last_time.month() as i32;
    let is_cross_month = current_month > previous_month;

    // 对 6 个基础桶计算差值
    let mut diff: HashMap<String, FlowDiff> = HashMap::new();

    for key in BUCKET_BASE {
        let cur_used = current.buckets.get(*key).map(|b| b.used).unwrap_or(0.0);
        let prev_used = previous.buckets.get(*key).map(|b| b.used).unwrap_or(0.0);

        let entry = if is_cross_month {
            // 跨月：流量已重置，直接使用当前值
            FlowDiff {
                uused: cur_used,
                today: cur_used,
            }
        } else {
            let uused = (cur_used - prev_used).max(0.0);
            let today = if today_str == last_date {
                // 同一天：累加今日用量
                let prev_today = previous.diff.get(*key).map(|d| d.today).unwrap_or(0.0);
                prev_today + uused
            } else {
                // 跨天：重置为本次用量
                uused
            };
            FlowDiff { uused, today }
        };

        diff.insert(key.to_string(), entry);
    }

    // 计算聚合桶的差值
    let common_keys = [
        "common_limited",
        "common_unlimited",
        "regional_limited",
        "regional_unlimited",
    ];
    let targeted_keys = ["targeted_limited", "targeted_unlimited"];

    let all_common_diff = sum_diff(&diff, &common_keys);
    let all_targeted_diff = sum_diff(&diff, &targeted_keys);
    let all_flow_diff = FlowDiff {
        uused: all_common_diff.uused + all_targeted_diff.uused,
        today: all_common_diff.today + all_targeted_diff.today,
    };

    diff.insert("所有通用".to_string(), all_common_diff);
    diff.insert("所有免流".to_string(), all_targeted_diff);
    diff.insert("所有流量".to_string(), all_flow_diff);

    diff
}

/// 求和 diff 中指定 keys 的 uused / today
fn sum_diff(diff: &HashMap<String, FlowDiff>, keys: &[&str]) -> FlowDiff {
    let mut result = FlowDiff::default();
    for key in keys {
        if let Some(d) = diff.get(*key) {
            result.uused += d.uused;
            result.today += d.today;
        }
    }
    result
}

// ─────────────────────────────────────────────
// 便捷方法
// ─────────────────────────────────────────────

/// 计算与上次查询的时间间隔（秒），并格式化为可读字符串
pub fn calc_time_interval(last_query_time: &str) -> String {
    if last_query_time.is_empty() {
        return String::new();
    }
    if let Ok(last) = NaiveDateTime::parse_from_str(last_query_time, "%Y-%m-%d %H:%M:%S") {
        let now = Local::now().naive_local();
        let diff_secs = (now - last).num_seconds();
        if diff_secs > 0 {
            return format_interval(diff_secs);
        }
    }
    String::new()
}

/// 设置余额信息
pub fn set_balance(analysis: &mut FlowAnalysis, balance: BalanceInfo) {
    analysis.balance = Some(balance);
}

/// 设置时间间隔
pub fn set_time_interval(analysis: &mut FlowAnalysis, last_query_time: &str) {
    analysis.time_interval = calc_time_interval(last_query_time);
}

// ─────────────────────────────────────────────
// 构建通知占位符
// ─────────────────────────────────────────────
// 辅助序列化字段重命名
// ─────────────────────────────────────────────
// 注意：FlowPackage 结构体使用 serde rename_all = "camelCase"
// 但 `type` 字段名保留为 "type"（serde 默认行为）
// `is_public_free` → `isPublicFree`
// `flow_type` → `flowType`
// `resource_type` → `resourceType`
// `end_date` → `endDate`
// `vice_card_list` → `viceCardList`
// `tag`, `name`, `total`, `use`, `remain`, `details` 直接对应

// ─────────────────────────────────────────────
// 单元测试
// ─────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_flow() {
        assert_eq!(format_flow(512.0), "512M");
        assert_eq!(format_flow(1024.0), "1G");
        assert_eq!(format_flow(1536.0), "1.5G");
        assert_eq!(format_flow(1048576.0), "1T");
        assert_eq!(format_flow(0.0), "0M");
        assert_eq!(format_flow(-1.0), "0");
    }

    #[test]
    fn test_format_interval() {
        assert_eq!(format_interval(0), "");
        assert_eq!(format_interval(65), "1分钟5秒");
        assert_eq!(format_interval(3661), "1小时1分钟");
        assert_eq!(format_interval(300), "5分钟");
        assert_eq!(format_interval(45), "45秒");
    }

    #[test]
    fn test_determine_package_type() {
        assert_eq!(determine_package_type("1", "01", false), "common");
        assert_eq!(determine_package_type("2", "01", false), "regional");
        assert_eq!(determine_package_type("3", "01", false), "regional");
        assert_eq!(determine_package_type("2", "13", false), "targeted");
        assert_eq!(determine_package_type("3", "I3", false), "targeted");
        assert_eq!(determine_package_type("1", "13", true), "targeted");
        // 默认归类
        assert_eq!(determine_package_type("1", "99", false), "common");
    }

    #[test]
    fn test_generate_full_buckets() {
        let mut buckets = HashMap::new();
        buckets.insert(
            "common_limited".to_string(),
            FlowBucket {
                total: 10240.0,
                used: 5120.0,
                remain: 5120.0,
            },
        );
        buckets.insert(
            "common_unlimited".to_string(),
            FlowBucket {
                total: 0.0,
                used: 2048.0,
                remain: -1.0,
            },
        );
        buckets.insert(
            "regional_limited".to_string(),
            FlowBucket {
                total: 2048.0,
                used: 512.0,
                remain: 1536.0,
            },
        );
        buckets.insert("regional_unlimited".to_string(), FlowBucket::default());
        buckets.insert(
            "targeted_limited".to_string(),
            FlowBucket {
                total: 4096.0,
                used: 1024.0,
                remain: 3072.0,
            },
        );
        buckets.insert("targeted_unlimited".to_string(), FlowBucket::default());

        let full = generate_full_buckets(&buckets);

        // 所有通用 = common + regional
        let all_common = full.get("所有通用").unwrap();
        assert_eq!(all_common.total, 12288.0); // 10240 + 2048
        assert_eq!(all_common.used, 7680.0); // 5120 + 2048 + 512
        assert_eq!(all_common.remain, 6656.0); // 5120 + (-1) + 1536 ← 无限桶只累加used

        // 所有免流
        let all_targeted = full.get("所有免流").unwrap();
        assert_eq!(all_targeted.total, 4096.0);
        assert_eq!(all_targeted.used, 1024.0);

        // 所有流量
        let all_flow = full.get("所有流量").unwrap();
        assert_eq!(all_flow.total, 16384.0);
        assert_eq!(all_flow.used, 8704.0);
    }

    #[test]
    fn test_calculate_diff_first_run() {
        let buckets = HashMap::new();
        let analysis = FlowAnalysis {
            timestamp: "2025-01-15 10:00:00".to_string(),
            main_package: "测试套餐".to_string(),
            packages: vec![],
            buckets,
            diff: HashMap::new(),
            time_interval: String::new(),
            balance: None,
        };
        let diff = calculate_diff(&analysis, "", "");
        assert!(!diff.is_empty());
    }

    #[test]
    fn test_is_current_mobile() {
        assert!(is_current_mobile("138****5678", "13812345678"));
        assert!(!is_current_mobile("139****5678", "13812345678"));
        assert!(is_current_mobile("13812345678", "13812345678"));
        assert!(!is_current_mobile("", "13812345678"));
    }

}
