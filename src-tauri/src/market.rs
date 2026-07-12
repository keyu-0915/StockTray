use std::collections::{HashMap, HashSet};

use chrono::{Local, NaiveDate, NaiveTime};
use serde::{Deserialize, Serialize};

use crate::{
    models::{
        MarketContribution, MarketDataQuality, MarketSnapshot, StockData, StyleAnalysis,
        SubsectorAnalysis,
    },
    quotes::{fetch_board_members, QuoteFetchResult},
};

pub(crate) const SAMPLE_VERSION: &str = "2026.07-v3";
pub(crate) const ALGORITHM_VERSION: &str = "2.0.1";
const MIN_UNIVERSE: usize = 450;
const MAX_UNIVERSE: usize = 550;
const MIN_PER_STYLE: usize = 120;
const MAX_PER_STYLE: usize = 180;
const MIN_COVERAGE: f32 = 80.0;
const LEADER_GAP: f32 = 8.0;
const STRONG_SCORE: f32 = 60.0;
const WEAK_SCORE: f32 = 40.0;

#[derive(Debug, Clone)]
struct BoardSpec {
    style: &'static str,
    subsector: &'static str,
    board: &'static str,
}

const BOARDS: &[BoardSpec] = &[
    BoardSpec {
        style: "young",
        subsector: "AI芯片",
        board: "BK1127",
    },
    BoardSpec {
        style: "young",
        subsector: "CPO",
        board: "BK1128",
    },
    BoardSpec {
        style: "young",
        subsector: "PCB",
        board: "BK0877",
    },
    BoardSpec {
        style: "young",
        subsector: "存储芯片",
        board: "BK1137",
    },
    BoardSpec {
        style: "young",
        subsector: "半导体设备",
        board: "BK1326",
    },
    BoardSpec {
        style: "middle",
        subsector: "商业航天",
        board: "BK0963",
    },
    BoardSpec {
        style: "middle",
        subsector: "游戏",
        board: "BK1046",
    },
    BoardSpec {
        style: "middle",
        subsector: "人形机器人",
        board: "BK1184",
    },
    BoardSpec {
        style: "middle",
        subsector: "机器人执行器",
        board: "BK1145",
    },
    BoardSpec {
        style: "old",
        subsector: "银行红利",
        board: "BK0475",
    },
    BoardSpec {
        style: "old",
        subsector: "保险",
        board: "BK0474",
    },
    BoardSpec {
        style: "old",
        subsector: "白色家电",
        board: "BK1239",
    },
    BoardSpec {
        style: "old",
        subsector: "食品饮料",
        board: "BK0438",
    },
    BoardSpec {
        style: "old",
        subsector: "煤炭",
        board: "BK0437",
    },
    BoardSpec {
        style: "old",
        subsector: "石油石化",
        board: "BK0464",
    },
];

const BROAD_INDICES: &[&str] = &["1.000001", "1.000300", "0.399001", "0.399006", "1.000688"];

pub(crate) fn index_secids() -> Vec<String> {
    BOARDS
        .iter()
        .map(|spec| format!("90.{}", spec.board))
        .chain(BROAD_INDICES.iter().map(|code| (*code).to_string()))
        .collect()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct Exposure {
    style: String,
    subsector: String,
    weight: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct SampleMember {
    pub(crate) code: String,
    pub(crate) name: String,
    exposures: Vec<Exposure>,
}

#[derive(Default)]
pub(crate) struct MarketEngine {
    pub(crate) members: Vec<SampleMember>,
    universe_source: String,
    last_amounts: HashMap<String, f64>,
    last_trading_date: String,
    pending_leader: Option<String>,
    pending_count: u8,
}

impl MarketEngine {
    pub(crate) async fn ensure_universe(&mut self) -> Result<(), String> {
        if !self.members.is_empty() {
            return Ok(());
        }
        if let Some(members) = load_cached_universe() {
            self.members = members;
            self.universe_source = "cache_exact".into();
            return Ok(());
        }
        self.members = match resolve_universe().await {
            Ok(members) => {
                save_cached_universe(&members);
                self.universe_source = "online_exact".into();
                members
            }
            Err(_) => {
                self.universe_source = "offline_proxy".into();
                embedded_universe()?
            }
        };
        Ok(())
    }

    pub(crate) fn codes(&self) -> Vec<String> {
        self.members
            .iter()
            .map(|member| member.code.clone())
            .collect()
    }

    pub(crate) fn analyze(
        &mut self,
        fetched: QuoteFetchResult,
        previous: Option<&MarketSnapshot>,
        refresh_minutes: u32,
    ) -> MarketSnapshot {
        let now = Local::now();
        self.analyze_at_date(
            fetched,
            previous,
            &now.format("%Y-%m-%d").to_string(),
            &now.format("%H:%M:%S").to_string(),
            refresh_minutes,
            true,
        )
    }

    #[cfg(test)]
    fn analyze_at(
        &mut self,
        fetched: QuoteFetchResult,
        previous: Option<&MarketSnapshot>,
        now_time: &str,
    ) -> MarketSnapshot {
        let now_date = fetched
            .quotes
            .iter()
            .find_map(|quote| (!quote.date.is_empty()).then_some(quote.date.as_str()))
            .unwrap_or("2026-07-10")
            .to_string();
        self.analyze_at_date(fetched, previous, &now_date, now_time, 15, false)
    }

    fn analyze_at_date(
        &mut self,
        fetched: QuoteFetchResult,
        previous: Option<&MarketSnapshot>,
        now_date: &str,
        now_time: &str,
        refresh_minutes: u32,
        enforce_index_quality: bool,
    ) -> MarketSnapshot {
        let expected = self.members.len();
        let all_quotes = fetched
            .quotes
            .into_iter()
            .filter(|quote| quote.error.is_empty() && quote.price > 0.0 && quote.prev_close > 0.0)
            .map(|quote| (quote.code.clone(), quote))
            .collect::<HashMap<_, _>>();
        let raw_received = all_quotes.len();
        let timestamp_missing = all_quotes
            .values()
            .filter(|quote| quote.date.is_empty() || quote.time.is_empty())
            .count();
        let trading_date = most_common_date(all_quotes.values()).unwrap_or_else(|| now_date.into());
        let raw_quotes = all_quotes
            .into_iter()
            .filter(|(_, quote)| !quote.date.is_empty() && !quote.time.is_empty())
            .collect::<HashMap<_, _>>();
        let data_time = raw_quotes
            .values()
            .map(|quote| quote.time.as_str())
            .filter(|time| !time.is_empty())
            .max()
            .unwrap_or(now_time);
        let max_delay_seconds = refresh_minutes.clamp(5, 30) as i64 * 120 + 60;
        let delayed_count = if trading_date == now_date && is_market_session_time(now_time) {
            raw_quotes
                .values()
                .filter(|quote| time_delay_seconds(now_time, &quote.time) > max_delay_seconds)
                .count()
        } else {
            0
        };
        let mut excluded_st = 0;
        let mut excluded_new = 0;
        let mut excluded_halted = 0;
        let quotes = raw_quotes
            .iter()
            .filter(|(_, quote)| match exclusion_reason(quote, &trading_date) {
                Some("st") => {
                    excluded_st += 1;
                    false
                }
                Some("new") => {
                    excluded_new += 1;
                    false
                }
                Some("halted") => {
                    excluded_halted += 1;
                    false
                }
                _ => true,
            })
            .map(|(code, quote)| (code.clone(), quote.clone()))
            .collect::<HashMap<_, _>>();
        let received = quotes.len();
        let coverage = if expected == 0 {
            0.0
        } else {
            received as f32 / expected as f32 * 100.0
        };
        let style_coverage = ["young", "middle", "old"]
            .into_iter()
            .map(|style| {
                let expected = self
                    .members
                    .iter()
                    .filter(|member| member.exposures.iter().any(|item| item.style == style))
                    .count();
                let received = self
                    .members
                    .iter()
                    .filter(|member| {
                        quotes.contains_key(&member.code)
                            && member.exposures.iter().any(|item| item.style == style)
                    })
                    .count();
                if expected == 0 {
                    0.0
                } else {
                    received as f32 / expected as f32 * 100.0
                }
            })
            .collect::<Vec<_>>();
        let minimum_style_coverage = style_coverage.iter().copied().fold(100.0, f32::min);
        if self.last_trading_date != trading_date {
            self.last_amounts.clear();
            self.pending_leader = None;
            self.pending_count = 0;
            self.last_trading_date = trading_date.clone();
        }
        let previous = previous.filter(|snapshot| {
            snapshot.trading_date == trading_date
                && snapshot.quality.sample_source == self.universe_source
        });
        let stale_count = raw_quotes
            .values()
            .filter(|quote| quote.date != trading_date)
            .count();
        let index_timestamp_missing = fetched
            .index_quotes
            .iter()
            .filter(|quote| quote.date.is_empty() || quote.time.is_empty())
            .count();
        let index_stale_count = fetched
            .index_quotes
            .iter()
            .filter(|quote| !quote.date.is_empty() && quote.date != trading_date)
            .count();
        let index_delayed_count = if trading_date == now_date && is_market_session_time(now_time) {
            fetched
                .index_quotes
                .iter()
                .filter(|quote| {
                    !quote.time.is_empty()
                        && time_delay_seconds(now_time, &quote.time) > max_delay_seconds
                })
                .count()
        } else {
            0
        };
        let valid_index_quotes = fetched
            .index_quotes
            .iter()
            .filter(|quote| {
                quote.error.is_empty()
                    && quote.price > 0.0
                    && quote.prev_close > 0.0
                    && quote.date == trading_date
                    && !quote.time.is_empty()
            })
            .cloned()
            .collect::<Vec<_>>();
        let index_expected = BOARDS.len() + BROAD_INDICES.len();
        let index_received = valid_index_quotes.len();
        let broad_index_received = valid_index_quotes
            .iter()
            .filter(|quote| is_broad_index(&quote.code))
            .count();
        let style_index_coverage = ["young", "middle", "old"]
            .into_iter()
            .map(|style| {
                let expected = BOARDS.iter().filter(|spec| spec.style == style).count();
                let received = BOARDS
                    .iter()
                    .filter(|spec| spec.style == style)
                    .filter(|spec| {
                        valid_index_quotes
                            .iter()
                            .any(|quote| quote.code == format!("em:{}", spec.board))
                    })
                    .count();
                received as f32 / expected.max(1) as f32 * 100.0
            })
            .collect::<Vec<_>>();
        let minimum_index_style_coverage =
            style_index_coverage.iter().copied().fold(100.0, f32::min);
        let index_evidence_ok = !enforce_index_quality
            || (fetched.index_error.is_empty()
                && index_timestamp_missing == 0
                && index_stale_count == 0
                && broad_index_received >= 3
                && minimum_index_style_coverage >= 60.0);
        let quality = MarketDataQuality {
            expected,
            received,
            coverage,
            mode: if fetched.fallback_count > 0 {
                "fallback"
            } else {
                "full"
            }
            .into(),
            sample_source: self.universe_source.clone(),
            style_coverage,
            minimum_style_coverage,
            raw_received,
            excluded_st,
            excluded_new,
            excluded_halted,
            timestamp_missing: timestamp_missing + index_timestamp_missing,
            delayed_count: delayed_count + index_delayed_count,
            index_expected,
            index_received,
            broad_index_received,
            style_index_coverage,
            index_error: fetched.index_error.clone(),
            primary_count: fetched.primary_count,
            fallback_count: fetched.fallback_count,
            stale_count: stale_count + index_stale_count,
            updated_at: data_time.into(),
        };

        let index_signals = style_index_signals(&valid_index_quotes);
        let market_return = broad_index_return(&valid_index_quotes, data_time);
        let metrics = build_metrics(
            &self.members,
            &quotes,
            &self.last_amounts,
            data_time,
            market_return,
        );
        let mut styles = ["young", "middle", "old"]
            .into_iter()
            .map(|style| {
                build_style(
                    style,
                    &metrics,
                    previous,
                    index_signals.get(style).copied().unwrap_or(0.0),
                )
            })
            .collect::<Vec<_>>();
        styles.sort_by_key(|style| style_order(&style.id));
        let (rotation_target, rotation_label, stability) = rotation_summary(&styles, previous);

        let observing = parse_time(now_time)
            .map(|time| time < NaiveTime::from_hms_opt(9, 45, 0).unwrap())
            .unwrap_or(false);
        let mut ranked = styles.iter().collect::<Vec<_>>();
        ranked.sort_by(|a, b| b.preference.total_cmp(&a.preference));
        let candidate = ranked.first().map(|style| style.id.clone());
        let gap = ranked
            .first()
            .zip(ranked.get(1))
            .map(|(a, b)| a.preference - b.preference)
            .unwrap_or(0.0);
        let strong_count = styles
            .iter()
            .filter(|style| style.score >= STRONG_SCORE)
            .count();
        let weak_count = styles
            .iter()
            .filter(|style| style.score <= WEAK_SCORE)
            .count();
        let candidate_is_strong = ranked
            .first()
            .is_some_and(|style| style.score >= STRONG_SCORE);
        let quality_ok = coverage >= MIN_COVERAGE
            && quality.minimum_style_coverage >= MIN_COVERAGE
            && quality.timestamp_missing == 0
            && quality.delayed_count == 0
            && quality.stale_count == 0
            && index_evidence_ok
            && expected > 0;
        let proxy_sample = quality.sample_source == "offline_proxy";

        let (status, leader) = if !quality_ok {
            self.pending_leader = None;
            self.pending_count = 0;
            ("no_conclusion".to_string(), None)
        } else if observing {
            (
                "observing".to_string(),
                previous.and_then(|snapshot| snapshot.leader.clone()),
            )
        } else if weak_count == styles.len() {
            self.pending_leader = None;
            self.pending_count = 0;
            ("all_weak".to_string(), None)
        } else if strong_count == styles.len() {
            self.pending_leader = None;
            self.pending_count = 0;
            ("broad_risk_on".to_string(), None)
        } else if strong_count >= 2 && gap < LEADER_GAP {
            self.pending_leader = None;
            self.pending_count = 0;
            ("co_strong".to_string(), None)
        } else if gap < LEADER_GAP || !candidate_is_strong {
            self.pending_leader = None;
            self.pending_count = 0;
            ("balanced".to_string(), None)
        } else {
            let candidate = candidate.unwrap_or_default();
            if self.pending_leader.as_deref() == Some(&candidate) {
                self.pending_count = self.pending_count.saturating_add(1);
            } else {
                self.pending_leader = Some(candidate.clone());
                self.pending_count = 1;
            }
            if previous.and_then(|snapshot| snapshot.leader.as_deref()) == Some(candidate.as_str())
                || self.pending_count >= 2
            {
                (
                    if proxy_sample { "proxy" } else { "dominant" }.to_string(),
                    Some(candidate),
                )
            } else {
                (
                    "forming".to_string(),
                    previous.and_then(|snapshot| snapshot.leader.clone()),
                )
            }
        };

        for quote in quotes.values() {
            self.last_amounts
                .insert(quote.code.clone(), quote.amount.max(0.0) as f64);
        }
        let leader_label = leader
            .as_deref()
            .map(style_label)
            .unwrap_or_else(|| match status.as_str() {
                "balanced" => "均衡",
                "all_weak" => "整体偏弱",
                "broad_risk_on" => "多线共强",
                "co_strong" => "双线共强",
                "observing" => "观察中",
                "no_conclusion" => "暂无结论",
                _ => "形成中",
            })
            .to_string();
        let consistency = leader
            .as_deref()
            .and_then(|id| styles.iter().find(|style| style.id == id))
            .map(|style| {
                if style.consistency >= 70.0 {
                    "较强"
                } else if style.consistency >= 55.0 {
                    "一般"
                } else {
                    "偏弱"
                }
            })
            .unwrap_or("-")
            .to_string();

        MarketSnapshot {
            trading_date,
            time: data_time.into(),
            status,
            leader,
            leader_label,
            signal_consistency: consistency,
            rotation_target,
            rotation_label,
            stability,
            quality,
            styles,
        }
    }
}

#[derive(Serialize, Deserialize)]
struct UniverseCache {
    sample_version: String,
    members: Vec<SampleMember>,
}

fn universe_cache_path() -> std::path::PathBuf {
    crate::config::config_path().with_file_name("market-universe.json")
}

fn load_cached_universe() -> Option<Vec<SampleMember>> {
    let cache: UniverseCache = std::fs::read_to_string(universe_cache_path())
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())?;
    (cache.sample_version == SAMPLE_VERSION && valid_universe(&cache.members))
        .then_some(cache.members)
}

fn embedded_universe() -> Result<Vec<SampleMember>, String> {
    let mut members = Vec::new();
    for line in include_str!("../resources/market-universe.txt")
        .lines()
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
    {
        let mut fields = line.split('|');
        let (Some(style), Some(subsector), Some(codes), None) =
            (fields.next(), fields.next(), fields.next(), fields.next())
        else {
            return Err("内置样本格式错误".into());
        };
        members.extend(codes.split(',').map(|code| SampleMember {
            code: code.into(),
            name: code.into(),
            exposures: vec![Exposure {
                style: style.into(),
                subsector: match subsector {
                    "AI硬件" => "电子代理",
                    "光通信" => "通信代理",
                    "算力基础设施" => "计算机代理",
                    "机器人" => "机械设备代理",
                    "游戏" => "传媒代理",
                    "商业航天" => "国防军工代理",
                    value => value,
                }
                .into(),
                weight: 1.0,
            }],
        }));
    }
    valid_universe(&members)
        .then_some(members)
        .ok_or_else(|| "内置样本未达到覆盖门槛".into())
}

fn valid_universe(members: &[SampleMember]) -> bool {
    if !(MIN_UNIVERSE..=MAX_UNIVERSE).contains(&members.len())
        || members
            .iter()
            .map(|member| &member.code)
            .collect::<HashSet<_>>()
            .len()
            != members.len()
    {
        return false;
    }
    ["young", "middle", "old"].into_iter().all(|style| {
        members
            .iter()
            .filter(|member| member.exposures.iter().any(|item| item.style == style))
            .count()
            >= MIN_PER_STYLE
    })
}

fn save_cached_universe(members: &[SampleMember]) {
    let path = universe_cache_path();
    let cache = UniverseCache {
        sample_version: SAMPLE_VERSION.into(),
        members: members.to_vec(),
    };
    if let (Some(parent), Ok(text)) = (path.parent(), serde_json::to_string_pretty(&cache)) {
        let _ = std::fs::create_dir_all(parent);
        let _ = crate::config::atomic_write(&path, text.as_bytes());
    }
}

#[derive(Clone)]
struct MemberMetric {
    code: String,
    name: String,
    exposure: Exposure,
    change_percent: f32,
    relative_return: f32,
    activity_component: f32,
    float_market_cap: f64,
    limit_state: i8,
}

async fn resolve_universe() -> Result<Vec<SampleMember>, String> {
    let mut groups = Vec::new();
    for spec in BOARDS {
        let members = fetch_board_members(spec.board)
            .await
            .map_err(|error| format!("{}: {error}", spec.subsector))?;
        groups.push((spec, members));
    }
    let members = build_stratified_universe(groups, MAX_PER_STYLE);
    valid_universe(&members)
        .then_some(members)
        .ok_or_else(|| "在线样本未达到总量或分类覆盖门槛".into())
}

fn build_stratified_universe(
    mut groups: Vec<(&BoardSpec, Vec<crate::quotes::BoardMember>)>,
    max_per_style: usize,
) -> Vec<SampleMember> {
    groups.sort_by_key(|(spec, _)| (spec.style, spec.subsector, spec.board));
    for (_, members) in &mut groups {
        members.sort_by(|a, b| a.code.cmp(&b.code));
        members.dedup_by(|a, b| a.code == b.code);
    }
    let mut raw: HashMap<String, (String, Vec<(String, String)>)> = HashMap::new();
    let mut style_codes: HashMap<&str, HashSet<String>> = HashMap::new();
    for style in ["young", "middle", "old"] {
        let style_groups = groups
            .iter()
            .filter(|(spec, _)| spec.style == style)
            .collect::<Vec<_>>();
        let max_len = style_groups
            .iter()
            .map(|(_, members)| members.len())
            .max()
            .unwrap_or(0);
        for index in 0..max_len {
            for (spec, members) in &style_groups {
                let Some(member) = members.get(index) else {
                    continue;
                };
                let style_set = style_codes.entry(spec.style).or_default();
                if !style_set.contains(&member.code) && style_set.len() >= max_per_style {
                    continue;
                }
                style_set.insert(member.code.clone());
                let entry = raw
                    .entry(member.code.clone())
                    .or_insert_with(|| (member.name.clone(), Vec::new()));
                if !entry
                    .1
                    .iter()
                    .any(|(style, subsector)| style == spec.style && subsector == spec.subsector)
                {
                    entry.1.push((spec.style.into(), spec.subsector.into()));
                }
            }
        }
    }
    let mut members = raw
        .into_iter()
        .map(|(code, (name, raw_exposures))| {
            let weight = 1.0 / raw_exposures.len() as f32;
            SampleMember {
                code,
                name,
                exposures: raw_exposures
                    .into_iter()
                    .map(|(style, subsector)| Exposure {
                        style,
                        subsector,
                        weight,
                    })
                    .collect(),
            }
        })
        .collect::<Vec<_>>();
    members.sort_by(|a, b| a.code.cmp(&b.code));
    members
}

fn build_metrics(
    members: &[SampleMember],
    quotes: &HashMap<String, StockData>,
    last_amounts: &HashMap<String, f64>,
    now_time: &str,
    broad_market_return: Option<f32>,
) -> Vec<MemberMetric> {
    let returns = quotes
        .values()
        .map(|quote| effective_return(quote, now_time))
        .collect::<Vec<_>>();
    let market_return = broad_market_return.unwrap_or_else(|| median(&returns));
    let mut board_returns: HashMap<&str, Vec<f32>> = HashMap::new();
    let mut caps = quotes
        .values()
        .filter(|quote| quote.float_market_cap > 0.0)
        .map(|quote| quote.float_market_cap)
        .collect::<Vec<_>>();
    caps.sort_by(f64::total_cmp);
    let low_cap = percentile_f64(&caps, 0.33);
    let high_cap = percentile_f64(&caps, 0.67);
    let mut cap_returns: [Vec<f32>; 3] = Default::default();
    for quote in quotes.values() {
        let value = effective_return(quote, now_time);
        board_returns
            .entry(listing_board(&quote.code))
            .or_default()
            .push(value);
        cap_returns[cap_bucket(quote.float_market_cap, low_cap, high_cap)].push(value);
    }
    let all_activity = quotes
        .values()
        .map(|quote| incremental_activity(quote, last_amounts))
        .filter(|value| *value > 0.0)
        .collect::<Vec<_>>();
    let median_activity = median(&all_activity).max(0.000_001);
    let activity_ready = !last_amounts.is_empty() && !all_activity.is_empty();
    let mut result = Vec::new();
    for member in members {
        let Some(quote) = quotes.get(&member.code) else {
            continue;
        };
        let own_return = effective_return(quote, now_time);
        let board_return = median(
            board_returns
                .get(listing_board(&quote.code))
                .map(Vec::as_slice)
                .unwrap_or(&[]),
        );
        let cap_return =
            median(&cap_returns[cap_bucket(quote.float_market_cap, low_cap, high_cap)]);
        let relative_return =
            own_return - (0.5 * market_return + 0.3 * board_return + 0.2 * cap_return);
        let activity_component = if activity_ready && last_amounts.contains_key(&quote.code) {
            let activity_ratio = incremental_activity(quote, last_amounts) / median_activity;
            (activity_ratio.max(0.01).ln() / 2.0).clamp(-1.0, 1.0)
        } else {
            0.0
        };
        for exposure in &member.exposures {
            result.push(MemberMetric {
                code: member.code.clone(),
                name: if quote.name.is_empty() {
                    member.name.clone()
                } else {
                    quote.name.clone()
                },
                exposure: exposure.clone(),
                change_percent: quote.change_percent,
                relative_return,
                activity_component,
                float_market_cap: quote.float_market_cap,
                limit_state: if quote.upper_limit > 0.0 && quote.price >= quote.upper_limit * 0.9995
                {
                    1
                } else if quote.lower_limit > 0.0 && quote.price <= quote.lower_limit * 1.0005 {
                    -1
                } else {
                    0
                },
            });
        }
    }
    result
}

fn build_style(
    style: &str,
    metrics: &[MemberMetric],
    previous: Option<&MarketSnapshot>,
    index_signal: f32,
) -> StyleAnalysis {
    let rows = metrics
        .iter()
        .filter(|metric| metric.exposure.style == style)
        .collect::<Vec<_>>();
    let weight_sum = rows
        .iter()
        .map(|row| row.exposure.weight)
        .sum::<f32>()
        .max(1.0);
    let mut subsector_breadth: HashMap<&str, (f32, f32)> = HashMap::new();
    for row in &rows {
        let entry = subsector_breadth
            .entry(&row.exposure.subsector)
            .or_default();
        entry.1 += row.exposure.weight;
        if row.change_percent > 0.01 {
            entry.0 += row.exposure.weight;
        } else if row.change_percent >= -0.01 {
            entry.0 += row.exposure.weight * 0.5;
        }
    }
    let mut contributions = rows
        .iter()
        .map(|row| {
            let weight = row.exposure.weight / weight_sum;
            let return_component = (row.relative_return / 3.0).tanh();
            let breadth_component = if row.relative_return > 0.0 { 1.0 } else { -1.0 };
            let confirmation_component = (index_signal / 2.0).clamp(-1.0, 1.0);
            let contribution = weight
                * (40.0 * return_component
                    + 25.0 * breadth_component
                    + 20.0 * row.activity_component
                    + 15.0 * confirmation_component);
            MarketContribution {
                code: row.code.clone(),
                name: row.name.clone(),
                subsector: row.exposure.subsector.clone(),
                contribution,
                change_percent: row.change_percent,
                reason: contribution_reason(row, index_signal),
            }
        })
        .collect::<Vec<_>>();
    let raw_score = contributions
        .iter()
        .map(|item| item.contribution)
        .sum::<f32>();
    let preference = (50.0 + raw_score / 2.0).clamp(0.0, 100.0);
    let relative_return = weighted_median(
        &rows
            .iter()
            .map(|row| (row.relative_return, row.exposure.weight))
            .collect::<Vec<_>>(),
    );
    let breadth = rows
        .iter()
        .map(|row| {
            if row.change_percent > 0.01 {
                row.exposure.weight
            } else if row.change_percent >= -0.01 {
                row.exposure.weight * 0.5
            } else {
                0.0
            }
        })
        .sum::<f32>()
        / weight_sum
        * 100.0;
    let activity = (50.0
        + rows
            .iter()
            .map(|row| row.activity_component * row.exposure.weight)
            .sum::<f32>()
            / weight_sum
            * 50.0)
        .clamp(0.0, 100.0);
    let subsector_confirmation = subsector_breadth
        .values()
        .filter(|(up, total)| *total > 0.0 && up / total >= 0.5)
        .count() as f32
        / subsector_breadth.len().max(1) as f32
        * 100.0;
    let confirmation =
        (0.5 * subsector_confirmation + 0.5 * (50.0 + index_signal * 15.0)).clamp(0.0, 100.0);
    let persistence = previous
        .and_then(|snapshot| snapshot.leader.as_deref())
        .map(|leader| if leader == style { 100.0 } else { 50.0 })
        .unwrap_or(50.0);
    let consistency = 0.4 * breadth + 0.3 * confirmation + 0.3 * persistence;
    let positive_sum = contributions
        .iter()
        .filter(|item| item.contribution > 0.0)
        .map(|item| item.contribution)
        .sum::<f32>();
    let negative_sum = contributions
        .iter()
        .filter(|item| item.contribution < 0.0)
        .map(|item| item.contribution.abs())
        .sum::<f32>();
    let (concentration, entropy) = distribution_metrics(
        &contributions
            .iter()
            .map(|item| item.contribution.abs())
            .collect::<Vec<_>>(),
    );
    let total_direction = positive_sum + negative_sum;
    let (direction, directional_share) = if total_direction <= f32::EPSILON {
        ("mixed", 0.0)
    } else if positive_sum >= negative_sum {
        (
            if positive_sum / total_direction >= 0.6 {
                "positive"
            } else {
                "mixed"
            },
            positive_sum / total_direction * 100.0,
        )
    } else {
        (
            if negative_sum / total_direction >= 0.6 {
                "negative"
            } else {
                "mixed"
            },
            negative_sum / total_direction * 100.0,
        )
    };
    let mut subsectors = subsector_breadth
        .into_iter()
        .map(|(name, (up, total))| SubsectorAnalysis {
            id: name.to_string(),
            name: name.to_string(),
            contribution: contributions
                .iter()
                .filter(|item| item.subsector == name)
                .map(|item| item.contribution)
                .sum(),
            breadth: if total > 0.0 { up / total * 100.0 } else { 0.0 },
        })
        .collect::<Vec<_>>();
    subsectors.sort_by(|a, b| b.contribution.abs().total_cmp(&a.contribution.abs()));
    contributions.sort_by(|a, b| b.contribution.total_cmp(&a.contribution));
    let positive = contributions
        .iter()
        .filter(|item| item.contribution > 0.0)
        .take(5)
        .cloned()
        .collect();
    let negative = contributions
        .iter()
        .rev()
        .filter(|item| item.contribution < 0.0)
        .take(5)
        .cloned()
        .collect();
    let heat = (50.0
        + weighted_median(
            &rows
                .iter()
                .map(|row| (row.change_percent, row.exposure.weight))
                .collect::<Vec<_>>(),
        ) * 8.0
        + (breadth - 50.0) * 0.3)
        .clamp(0.0, 100.0);
    let score =
        (0.55 * heat + 0.2 * activity + 0.15 * confirmation + 0.1 * consistency).clamp(0.0, 100.0);
    let dominant_positive = positive_sum >= negative_sum;
    let diffusion = subsectors
        .iter()
        .filter(|subsector| {
            if dominant_positive {
                subsector.contribution > 0.0 && subsector.breadth >= 60.0
            } else {
                subsector.contribution < 0.0 && subsector.breadth <= 40.0
            }
        })
        .count() as f32
        / subsectors.len().max(1) as f32
        * 100.0;
    let equal_weight_return = rows
        .iter()
        .map(|row| row.relative_return * row.exposure.weight)
        .sum::<f32>()
        / weight_sum;
    let cap_weight_sum = rows
        .iter()
        .map(|row| row.float_market_cap.max(0.0) * row.exposure.weight as f64)
        .sum::<f64>();
    let cap_weight_return = if cap_weight_sum > 0.0 {
        (rows
            .iter()
            .map(|row| {
                row.relative_return as f64
                    * row.float_market_cap.max(0.0)
                    * row.exposure.weight as f64
            })
            .sum::<f64>()
            / cap_weight_sum) as f32
    } else {
        equal_weight_return
    };
    let score_change = previous
        .and_then(|snapshot| snapshot.styles.iter().find(|item| item.id == style))
        .map(|old| score - old.score)
        .unwrap_or(0.0);
    StyleAnalysis {
        id: style.into(),
        label: style_label(style).into(),
        subtitle: style_subtitle(style).into(),
        score,
        heat,
        preference,
        state: if score >= STRONG_SCORE {
            "strong"
        } else if score <= WEAK_SCORE {
            "weak"
        } else {
            "neutral"
        }
        .into(),
        score_change,
        relative_return,
        breadth,
        activity,
        confirmation,
        consistency,
        concentration,
        entropy,
        diffusion,
        direction: direction.into(),
        directional_share,
        equal_weight_return,
        cap_weight_return,
        weighting_divergence: cap_weight_return - equal_weight_return,
        subsectors,
        positive,
        negative,
    }
}

fn exclusion_reason(quote: &StockData, trading_date: &str) -> Option<&'static str> {
    if quote.name.to_ascii_uppercase().contains("ST") {
        return Some("st");
    }
    let prefixed_new = quote.name.starts_with('N') || quote.name.starts_with('C');
    let recently_listed = NaiveDate::parse_from_str(&quote.listing_date, "%Y-%m-%d")
        .ok()
        .zip(NaiveDate::parse_from_str(trading_date, "%Y-%m-%d").ok())
        .is_some_and(|(listed, traded)| (0..30).contains(&(traded - listed).num_days()));
    if prefixed_new || recently_listed {
        return Some("new");
    }
    if quote.volume <= 0.0 && quote.amount <= 0.0 {
        return Some("halted");
    }
    None
}

fn contribution_reason(metric: &MemberMetric, index_signal: f32) -> String {
    let mut reasons = Vec::new();
    if metric.limit_state > 0 {
        reasons.push("涨停");
    } else if metric.limit_state < 0 {
        reasons.push("跌停");
    }
    if metric.relative_return >= 0.5 {
        reasons.push("相对收益领先");
    } else if metric.relative_return <= -0.5 {
        reasons.push("相对收益落后");
    }
    if metric.activity_component >= 0.25 {
        reasons.push("增量成交活跃");
    } else if metric.activity_component <= -0.25 {
        reasons.push("增量成交降温");
    }
    if index_signal >= 0.5 {
        reasons.push("板块指数确认");
    } else if index_signal <= -0.5 {
        reasons.push("板块指数拖累");
    }
    if reasons.is_empty() {
        reasons.push(if metric.change_percent >= 0.0 {
            "成分表现偏强"
        } else {
            "成分表现偏弱"
        });
    }
    reasons.join(" · ")
}

fn distribution_metrics(values: &[f32]) -> (f32, f32) {
    let values = values
        .iter()
        .copied()
        .filter(|value| *value > f32::EPSILON)
        .collect::<Vec<_>>();
    let total = values.iter().sum::<f32>();
    if values.is_empty() || total <= f32::EPSILON {
        return (0.0, 0.0);
    }
    let concentration = values
        .iter()
        .map(|value| (value / total).powi(2))
        .sum::<f32>()
        * 100.0;
    let entropy = if values.len() == 1 {
        0.0
    } else {
        -values
            .iter()
            .map(|value| {
                let share = value / total;
                share * share.ln()
            })
            .sum::<f32>()
            / (values.len() as f32).ln()
            * 100.0
    };
    (concentration, entropy)
}

fn rotation_summary(
    styles: &[StyleAnalysis],
    previous: Option<&MarketSnapshot>,
) -> (Option<String>, String, f32) {
    if previous.is_none() {
        return (None, "等待下一快照".into(), 50.0);
    }
    let mean_change = styles
        .iter()
        .map(|style| style.score_change.abs())
        .sum::<f32>()
        / styles.len().max(1) as f32;
    let stability = (100.0 - mean_change * 8.0).clamp(0.0, 100.0);
    if styles.iter().all(|style| style.score_change >= 2.0) {
        return (None, "多线同步走强".into(), stability);
    }
    if styles.iter().all(|style| style.score_change <= -2.0) {
        return (None, "多线同步走弱".into(), stability);
    }
    let mut ranked = styles.iter().collect::<Vec<_>>();
    ranked.sort_by(|a, b| b.score_change.total_cmp(&a.score_change));
    let first = ranked[0];
    let second = ranked.get(1).map_or(0.0, |style| style.score_change);
    if first.score_change >= 3.0 && first.score_change - second >= 1.0 {
        return (
            Some(first.id.clone()),
            format!("向{}增强", first.label),
            stability,
        );
    }
    (None, "轮动不明显".into(), stability)
}

fn style_index_signals(quotes: &[StockData]) -> HashMap<String, f32> {
    let quote_by_board = quotes
        .iter()
        .filter_map(|quote| {
            quote
                .code
                .strip_prefix("em:")
                .map(|code| (code, quote.change_percent))
        })
        .collect::<HashMap<_, _>>();
    ["young", "middle", "old"]
        .into_iter()
        .map(|style| {
            let values = BOARDS
                .iter()
                .filter(|spec| spec.style == style)
                .filter_map(|spec| quote_by_board.get(spec.board).copied())
                .collect::<Vec<_>>();
            (style.to_string(), median(&values))
        })
        .collect()
}

fn broad_index_return(quotes: &[StockData], now_time: &str) -> Option<f32> {
    let values = quotes
        .iter()
        .filter(|quote| is_broad_index(&quote.code))
        .map(|quote| effective_return(quote, now_time))
        .collect::<Vec<_>>();
    (!values.is_empty()).then(|| median(&values))
}

fn effective_return(quote: &StockData, now_time: &str) -> f32 {
    if quote.prev_close <= 0.0 || quote.price <= 0.0 {
        return 0.0;
    }
    let gap = if quote.open > 0.0 {
        (quote.open / quote.prev_close - 1.0) * 100.0
    } else {
        0.0
    };
    let intraday = if quote.open > 0.0 {
        (quote.price / quote.open - 1.0) * 100.0
    } else {
        quote.change_percent
    };
    let time = parse_time(now_time);
    let gap_weight = if time
        .map(|t| t < NaiveTime::from_hms_opt(10, 0, 0).unwrap())
        .unwrap_or(false)
    {
        0.3
    } else {
        0.2
    };
    gap_weight * gap + (1.0 - gap_weight) * intraday
}

fn incremental_activity(quote: &StockData, last_amounts: &HashMap<String, f64>) -> f32 {
    if quote.float_market_cap <= 0.0 {
        return 0.0;
    }
    let Some(previous) = last_amounts.get(&quote.code).copied() else {
        return 0.0;
    };
    ((quote.amount as f64 - previous).max(0.0) / quote.float_market_cap) as f32
}

fn listing_board(code: &str) -> &'static str {
    if code.starts_with("sh688") {
        "star"
    } else if code.starts_with("sz300") {
        "chinext"
    } else if code.starts_with("sh") {
        "sh"
    } else {
        "sz"
    }
}

fn cap_bucket(cap: f64, low: f64, high: f64) -> usize {
    if cap <= 0.0 || cap < low {
        0
    } else if cap < high {
        1
    } else {
        2
    }
}

fn parse_time(value: &str) -> Option<NaiveTime> {
    NaiveTime::parse_from_str(value, "%H:%M:%S").ok()
}

fn is_market_session_time(value: &str) -> bool {
    parse_time(value).is_some_and(|time| {
        let morning_start = NaiveTime::from_hms_opt(9, 30, 0).unwrap();
        let morning_end = NaiveTime::from_hms_opt(11, 31, 0).unwrap();
        let afternoon_start = NaiveTime::from_hms_opt(13, 0, 0).unwrap();
        let afternoon_end = NaiveTime::from_hms_opt(15, 6, 0).unwrap();
        (morning_start..=morning_end).contains(&time)
            || (afternoon_start..=afternoon_end).contains(&time)
    })
}

fn time_delay_seconds(now: &str, quote_time: &str) -> i64 {
    parse_time(now)
        .zip(parse_time(quote_time))
        .map(|(now, quote)| (now - quote).num_seconds().max(0))
        .unwrap_or(i64::MAX)
}

fn is_broad_index(code: &str) -> bool {
    code.strip_prefix("em:")
        .is_some_and(|code| BROAD_INDICES.iter().any(|index| index.ends_with(code)))
}

fn most_common_date<'a>(quotes: impl Iterator<Item = &'a StockData>) -> Option<String> {
    let mut counts = HashMap::new();
    for date in quotes
        .map(|quote| quote.date.as_str())
        .filter(|date| !date.is_empty())
    {
        *counts.entry(date).or_insert(0usize) += 1;
    }
    counts
        .into_iter()
        .max_by_key(|(_, count)| *count)
        .map(|(date, _)| date.to_string())
}

fn median(values: &[f32]) -> f32 {
    if values.is_empty() {
        return 0.0;
    }
    let mut values = values.to_vec();
    values.sort_by(f32::total_cmp);
    if values.len().is_multiple_of(2) {
        (values[values.len() / 2 - 1] + values[values.len() / 2]) / 2.0
    } else {
        values[values.len() / 2]
    }
}

fn weighted_median(values: &[(f32, f32)]) -> f32 {
    if values.is_empty() {
        return 0.0;
    }
    let mut values = values.to_vec();
    values.sort_by(|a, b| a.0.total_cmp(&b.0));
    let half = values
        .iter()
        .map(|(_, weight)| weight.max(0.0))
        .sum::<f32>()
        / 2.0;
    let mut cumulative = 0.0;
    for (value, weight) in values {
        cumulative += weight.max(0.0);
        if cumulative >= half {
            return value;
        }
    }
    0.0
}

fn percentile_f64(values: &[f64], percentile: f32) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    values[((values.len() - 1) as f32 * percentile).round() as usize]
}

fn style_order(style: &str) -> u8 {
    match style {
        "young" => 0,
        "middle" => 1,
        _ => 2,
    }
}
fn style_label(style: &str) -> &'static str {
    match style {
        "young" => "小登",
        "middle" => "中登",
        "old" => "老登",
        _ => "-",
    }
}
fn style_subtitle(style: &str) -> &'static str {
    match style {
        "young" => "AI硬件",
        "middle" => "商业航天 · 游戏 · 机器人",
        "old" => "银行红利 · 消费 · 资源",
        _ => "",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn member(code: &str, style: &str, subsector: &str) -> SampleMember {
        SampleMember {
            code: code.into(),
            name: code.into(),
            exposures: vec![Exposure {
                style: style.into(),
                subsector: subsector.into(),
                weight: 1.0,
            }],
        }
    }

    fn quote(code: &str, change: f32, amount: f32) -> StockData {
        StockData {
            code: code.into(),
            name: code.into(),
            price: 100.0 * (1.0 + change / 100.0),
            prev_close: 100.0,
            open: 100.0,
            change_percent: change,
            amount,
            float_market_cap: 1_000_000_000.0,
            date: "2026-07-10".into(),
            time: "11:00:00".into(),
            source: "test".into(),
            ..Default::default()
        }
    }

    fn complete_index_evidence(time: &str) -> Vec<StockData> {
        BOARDS
            .iter()
            .map(|spec| format!("em:{}", spec.board))
            .chain(
                BROAD_INDICES
                    .iter()
                    .map(|code| format!("em:{}", code.split('.').next_back().unwrap())),
            )
            .map(|code| {
                let mut quote = quote(&code, 0.0, 0.0);
                quote.time = time.into();
                quote
            })
            .collect()
    }

    #[test]
    fn requires_two_snapshots_before_confirming_a_new_leader() {
        let mut engine = MarketEngine {
            members: vec![
                member("sh600001", "young", "AI芯片"),
                member("sh600002", "middle", "商业航天"),
                member("sh600003", "old", "银行红利"),
            ],
            ..Default::default()
        };
        let fetched = || QuoteFetchResult {
            quotes: vec![
                quote("sh600001", 0.2, 1_000.0),
                quote("sh600002", 5.0, 10_000.0),
                quote("sh600003", -1.0, 500.0),
            ],
            index_quotes: Vec::new(),
            index_error: String::new(),
            primary_count: 3,
            fallback_count: 0,
        };
        let first = engine.analyze_at(fetched(), None, "10:30:00");
        assert_eq!(first.status, "forming");
        let second = engine.analyze_at(fetched(), Some(&first), "10:45:00");
        assert_eq!(second.leader.as_deref(), Some("middle"));
    }

    #[test]
    fn low_coverage_never_outputs_a_conclusion() {
        let mut engine = MarketEngine {
            members: vec![
                member("a", "young", "AI芯片"),
                member("b", "middle", "游戏"),
            ],
            ..Default::default()
        };
        let snapshot = engine.analyze_at(
            QuoteFetchResult {
                quotes: vec![quote("a", 3.0, 1_000.0)],
                index_quotes: Vec::new(),
                index_error: String::new(),
                primary_count: 1,
                fallback_count: 0,
            },
            None,
            "11:00:00",
        );
        assert_eq!(snapshot.status, "no_conclusion");
        assert!(snapshot.leader.is_none());
    }

    #[test]
    fn weak_single_style_coverage_blocks_an_overall_healthy_sample() {
        let mut members = (0..8)
            .map(|index| member(&format!("y{index}"), "young", "AI芯片"))
            .collect::<Vec<_>>();
        members.push(member("m", "middle", "游戏"));
        members.push(member("o", "old", "银行红利"));
        let mut engine = MarketEngine {
            members,
            ..Default::default()
        };
        let mut quotes = (0..8)
            .map(|index| quote(&format!("y{index}"), 1.0, 1_000.0))
            .collect::<Vec<_>>();
        quotes.push(quote("m", 1.0, 1_000.0));
        let snapshot = engine.analyze_at(
            QuoteFetchResult {
                quotes,
                index_quotes: Vec::new(),
                index_error: String::new(),
                primary_count: 9,
                fallback_count: 0,
            },
            None,
            "11:00:00",
        );
        assert_eq!(snapshot.quality.coverage, 90.0);
        assert_eq!(snapshot.quality.minimum_style_coverage, 0.0);
        assert_eq!(snapshot.status, "no_conclusion");
    }

    #[test]
    fn missing_or_delayed_index_evidence_blocks_a_conclusion() {
        let members = vec![
            member("y", "young", "AI芯片"),
            member("m", "middle", "游戏"),
            member("o", "old", "银行红利"),
        ];
        let stock_quotes = || {
            vec![
                quote("y", 0.2, 1_000.0),
                quote("m", 5.0, 10_000.0),
                quote("o", -1.0, 500.0),
            ]
        };
        let mut engine = MarketEngine {
            members: members.clone(),
            ..Default::default()
        };
        let missing = engine.analyze_at_date(
            QuoteFetchResult {
                quotes: stock_quotes(),
                index_quotes: Vec::new(),
                index_error: "index unavailable".into(),
                primary_count: 3,
                fallback_count: 0,
            },
            None,
            "2026-07-10",
            "11:00:00",
            5,
            true,
        );
        assert_eq!(missing.status, "no_conclusion");
        assert_eq!(missing.quality.index_received, 0);
        assert_eq!(missing.quality.index_error, "index unavailable");

        let mut delayed_stocks = stock_quotes();
        for quote in &mut delayed_stocks {
            quote.time = "09:30:00".into();
        }
        let mut engine = MarketEngine {
            members,
            ..Default::default()
        };
        let delayed = engine.analyze_at_date(
            QuoteFetchResult {
                quotes: delayed_stocks,
                index_quotes: complete_index_evidence("09:30:00"),
                index_error: String::new(),
                primary_count: 3,
                fallback_count: 0,
            },
            None,
            "2026-07-10",
            "11:00:00",
            5,
            true,
        );
        assert_eq!(delayed.status, "no_conclusion");
        assert!(delayed.quality.delayed_count > 0);
    }

    #[test]
    fn complete_same_day_index_evidence_passes_the_quality_gate() {
        let mut engine = MarketEngine {
            members: vec![
                member("y", "young", "AI芯片"),
                member("m", "middle", "游戏"),
                member("o", "old", "银行红利"),
            ],
            ..Default::default()
        };
        let snapshot = engine.analyze_at_date(
            QuoteFetchResult {
                quotes: vec![
                    quote("y", 0.2, 1_000.0),
                    quote("m", 5.0, 10_000.0),
                    quote("o", -1.0, 500.0),
                ],
                index_quotes: complete_index_evidence("11:00:00"),
                index_error: String::new(),
                primary_count: 3,
                fallback_count: 0,
            },
            None,
            "2026-07-10",
            "11:00:00",
            15,
            true,
        );
        assert_ne!(snapshot.status, "no_conclusion");
        assert_eq!(snapshot.quality.index_received, index_secids().len());
        assert_eq!(snapshot.quality.broad_index_received, BROAD_INDICES.len());
    }

    #[test]
    fn offline_proxy_never_claims_a_dominant_style() {
        let mut engine = MarketEngine {
            members: vec![
                member("y", "young", "电子代理"),
                member("m", "middle", "传媒代理"),
                member("o", "old", "银行红利"),
            ],
            universe_source: "offline_proxy".into(),
            ..Default::default()
        };
        let fetched = || QuoteFetchResult {
            quotes: vec![
                quote("y", 0.2, 1_000.0),
                quote("m", 5.0, 10_000.0),
                quote("o", -1.0, 500.0),
            ],
            index_quotes: Vec::new(),
            index_error: String::new(),
            primary_count: 3,
            fallback_count: 0,
        };
        let first = engine.analyze_at(fetched(), None, "10:30:00");
        let second = engine.analyze_at(fetched(), Some(&first), "10:45:00");
        assert_eq!(second.status, "proxy");
        assert_eq!(second.leader.as_deref(), Some("middle"));
    }

    #[test]
    fn a_new_trading_date_requires_fresh_leader_confirmation() {
        let mut engine = MarketEngine {
            members: vec![
                member("y", "young", "AI芯片"),
                member("m", "middle", "游戏"),
                member("o", "old", "银行红利"),
            ],
            ..Default::default()
        };
        let fetched = |date: &str| {
            let mut quotes = vec![
                quote("y", 0.2, 1_000.0),
                quote("m", 5.0, 10_000.0),
                quote("o", -1.0, 500.0),
            ];
            for quote in &mut quotes {
                quote.date = date.into();
            }
            QuoteFetchResult {
                quotes,
                index_quotes: Vec::new(),
                index_error: String::new(),
                primary_count: 3,
                fallback_count: 0,
            }
        };
        let first = engine.analyze_at(fetched("2026-07-10"), None, "10:30:00");
        let second = engine.analyze_at(fetched("2026-07-10"), Some(&first), "10:45:00");
        assert_eq!(second.status, "dominant");
        let next_day = engine.analyze_at(fetched("2026-07-13"), Some(&second), "10:30:00");
        assert_eq!(next_day.status, "forming");
        assert!(next_day.leader.is_none());
    }

    #[test]
    fn subsector_contributions_reconcile_to_relative_preference() {
        let rows = vec![MemberMetric {
            code: "a".into(),
            name: "a".into(),
            exposure: Exposure {
                style: "middle".into(),
                subsector: "游戏".into(),
                weight: 1.0,
            },
            change_percent: 3.0,
            relative_return: 2.0,
            activity_component: 0.5,
            float_market_cap: 1_000_000_000.0,
            limit_state: 0,
        }];
        let style = build_style("middle", &rows, None, 1.0);
        let subtotal: f32 = style.subsectors.iter().map(|item| item.contribution).sum();
        assert!((style.preference - (50.0 + subtotal / 2.0)).abs() < 0.001);
    }

    #[test]
    fn independent_strength_is_not_the_same_as_relative_preference() {
        let rows = vec![MemberMetric {
            code: "a".into(),
            name: "a".into(),
            exposure: Exposure {
                style: "young".into(),
                subsector: "AI芯片".into(),
                weight: 1.0,
            },
            change_percent: 5.0,
            relative_return: 0.0,
            activity_component: 0.0,
            float_market_cap: 1_000_000_000.0,
            limit_state: 0,
        }];
        let style = build_style("young", &rows, None, 0.0);
        assert_eq!(style.state, "strong");
        assert!(style.score >= STRONG_SCORE);
        assert!(style.preference < 50.0);
    }

    #[test]
    fn explanation_metrics_detect_distribution_and_cap_bias() {
        let (concentration, entropy) = distribution_metrics(&[1.0, 1.0]);
        assert!((concentration - 50.0).abs() < 0.001);
        assert!((entropy - 100.0).abs() < 0.001);
        let rows = vec![
            MemberMetric {
                code: "small".into(),
                name: "small".into(),
                exposure: Exposure {
                    style: "old".into(),
                    subsector: "银行红利".into(),
                    weight: 1.0,
                },
                change_percent: -1.0,
                relative_return: -1.0,
                activity_component: 0.0,
                float_market_cap: 1.0,
                limit_state: 0,
            },
            MemberMetric {
                code: "large".into(),
                name: "large".into(),
                exposure: Exposure {
                    style: "old".into(),
                    subsector: "银行红利".into(),
                    weight: 1.0,
                },
                change_percent: 1.0,
                relative_return: 1.0,
                activity_component: 0.5,
                float_market_cap: 9.0,
                limit_state: 1,
            },
        ];
        let style = build_style("old", &rows, None, 0.0);
        assert!(style.weighting_divergence > 0.7);
        assert!(style
            .positive
            .iter()
            .any(|item| item.reason.contains("相对收益领先")));
        assert!(style
            .positive
            .iter()
            .any(|item| item.reason.contains("涨停")));
    }

    #[test]
    fn abnormal_samples_are_classified_before_scoring() {
        let mut st = quote("st", 1.0, 1_000.0);
        st.name = "*ST样本".into();
        let mut new_stock = quote("new", 1.0, 1_000.0);
        new_stock.listing_date = "2026-07-01".into();
        let halted = quote("halted", 0.0, 0.0);
        let normal = quote("normal", 1.0, 1_000.0);
        assert_eq!(exclusion_reason(&st, "2026-07-10"), Some("st"));
        assert_eq!(exclusion_reason(&new_stock, "2026-07-10"), Some("new"));
        assert_eq!(exclusion_reason(&halted, "2026-07-10"), Some("halted"));
        assert_eq!(exclusion_reason(&normal, "2026-07-10"), None);
    }

    #[test]
    fn overall_state_recognizes_co_strength_and_all_weakness() {
        let analyze = |changes: [f32; 3]| {
            let mut engine = MarketEngine {
                members: vec![
                    member("y", "young", "AI芯片"),
                    member("m", "middle", "游戏"),
                    member("o", "old", "银行红利"),
                ],
                ..Default::default()
            };
            engine.analyze_at(
                QuoteFetchResult {
                    quotes: vec![
                        quote("y", changes[0], 1_000.0),
                        quote("m", changes[1], 1_000.0),
                        quote("o", changes[2], 1_000.0),
                    ],
                    index_quotes: Vec::new(),
                    index_error: String::new(),
                    primary_count: 3,
                    fallback_count: 0,
                },
                None,
                "11:00:00",
            )
        };
        assert_eq!(analyze([5.0, 5.0, -5.0]).status, "co_strong");
        assert_eq!(analyze([-5.0, -5.0, -5.0]).status, "all_weak");
        assert_eq!(analyze([5.0, 5.0, 5.0]).status, "broad_risk_on");
    }

    #[test]
    fn rotation_reports_a_clear_strengthening_target() {
        let styles = [
            StyleAnalysis {
                id: "young".into(),
                label: "小登".into(),
                score_change: 5.0,
                ..Default::default()
            },
            StyleAnalysis {
                id: "middle".into(),
                label: "中登".into(),
                score_change: 1.0,
                ..Default::default()
            },
            StyleAnalysis {
                id: "old".into(),
                label: "老登".into(),
                score_change: -1.0,
                ..Default::default()
            },
        ];
        let (target, label, stability) =
            rotation_summary(&styles, Some(&MarketSnapshot::default()));
        assert_eq!(target.as_deref(), Some("young"));
        assert_eq!(label, "向小登增强");
        assert!(stability < 100.0);
    }

    #[test]
    fn activity_is_neutral_without_a_comparable_previous_sample() {
        let members = vec![member("a", "young", "AI芯片")];
        let quotes = [("a".into(), quote("a", 1.0, 1_000.0))]
            .into_iter()
            .collect();
        let rows = build_metrics(&members, &quotes, &HashMap::new(), "10:30:00", None);
        assert_eq!(rows[0].activity_component, 0.0);
    }

    #[test]
    fn broad_indices_supply_the_market_benchmark() {
        let mut broad = quote("em:000300", 2.0, 0.0);
        broad.open = broad.prev_close;
        let mut board = quote("em:BK1127", 9.0, 0.0);
        board.open = board.prev_close;
        let value = broad_index_return(&[broad, board], "11:00:00").unwrap();
        assert!((value - 1.6).abs() < 0.001);
    }

    #[test]
    fn embedded_universe_has_balanced_offline_coverage() {
        let members = embedded_universe().unwrap();
        assert_eq!(members.len(), 450);
        for style in ["young", "middle", "old"] {
            assert_eq!(
                members
                    .iter()
                    .filter(|member| member.exposures[0].style == style)
                    .count(),
                150
            );
        }
        assert!(members
            .iter()
            .flat_map(|member| &member.exposures)
            .any(|exposure| exposure.subsector == "电子代理"));
    }

    #[test]
    fn stratified_universe_is_deterministic_and_balances_subsectors() {
        let member = |code: &str| crate::quotes::BoardMember {
            code: code.into(),
            name: code.into(),
        };
        let first = build_stratified_universe(
            vec![
                (
                    &BOARDS[0],
                    vec![member("sh600003"), member("sh600001"), member("sh600002")],
                ),
                (
                    &BOARDS[1],
                    vec![member("sh600006"), member("sh600004"), member("sh600005")],
                ),
            ],
            4,
        );
        let second = build_stratified_universe(
            vec![
                (
                    &BOARDS[1],
                    vec![member("sh600005"), member("sh600006"), member("sh600004")],
                ),
                (
                    &BOARDS[0],
                    vec![member("sh600002"), member("sh600001"), member("sh600003")],
                ),
            ],
            4,
        );
        assert_eq!(
            serde_json::to_string(&first).unwrap(),
            serde_json::to_string(&second).unwrap()
        );
        assert_eq!(first.len(), 4);
        assert!(BOARDS[..2].iter().all(|spec| first.iter().any(|item| item
            .exposures
            .iter()
            .any(|exposure| exposure.subsector == spec.subsector))));
    }
}
