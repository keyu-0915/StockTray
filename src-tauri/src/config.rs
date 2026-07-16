use std::{
    collections::HashSet,
    fs::{self, File},
    io::Write,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use chrono::{Datelike, Local};

use crate::models::{
    default_background_refresh_interval_ms, default_display_fields, default_theme,
    default_tooltip_fields, AppConfig, AppearanceConfig, MarketAnalysisConfig, MarketAnalysisState,
    MarketDayArchive, MarketDaySummary, MarketHistoryStore, MarketStorageInfo, PopupConfig,
    StockEntry, CURRENT_CONFIG_SCHEMA_VERSION,
};

pub(crate) fn config_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_default())
        .join("StockTray")
        .join("config.json")
}

pub(crate) fn market_state_path() -> PathBuf {
    config_path().with_file_name("market-snapshots.json")
}

pub(crate) fn market_history_path() -> PathBuf {
    config_path().with_file_name("market-history.json")
}

pub(crate) fn load_market_history() -> MarketHistoryStore {
    let mut store = fs::read_to_string(market_history_path())
        .ok()
        .and_then(|text| serde_json::from_str::<MarketHistoryStore>(&text).ok())
        .unwrap_or_default();
    store.days.retain(|day| {
        !day.trading_date.is_empty() && day.snapshot.trading_date == day.trading_date
    });
    store
        .days
        .sort_by(|a, b| a.trading_date.cmp(&b.trading_date));
    store.days.dedup_by(|a, b| {
        if a.trading_date == b.trading_date {
            *a = b.clone();
            true
        } else {
            false
        }
    });
    store.schema_version = crate::models::default_market_history_schema_version();
    store
}

pub(crate) fn save_market_history(store: &MarketHistoryStore) -> Result<(), String> {
    let mut normalized = store.clone();
    normalized.schema_version = crate::models::default_market_history_schema_version();
    atomic_write(
        &market_history_path(),
        serde_json::to_vec(&normalized)
            .map_err(|error| error.to_string())?
            .as_slice(),
    )
}

pub(crate) fn archive_market_day(state: &MarketAnalysisState) -> Result<bool, String> {
    let Some(snapshot) = state.current.as_ref() else {
        return Ok(false);
    };
    if snapshot.trading_date.is_empty() {
        return Ok(false);
    }
    let mut store = load_market_history();
    let day = MarketDayArchive {
        trading_date: snapshot.trading_date.clone(),
        sample_version: state.sample_version.clone(),
        algorithm_version: state.algorithm_version.clone(),
        snapshot: snapshot.clone(),
        history: state.history.clone(),
    };
    if let Some(existing) = store
        .days
        .iter_mut()
        .find(|item| item.trading_date == day.trading_date)
    {
        existing.snapshot = day.snapshot;
        existing.sample_version = day.sample_version;
        existing.algorithm_version = day.algorithm_version;
        existing.history.extend(day.history);
        existing.history.sort_by(|a, b| a.time.cmp(&b.time));
        existing.history.dedup_by(|a, b| a.time == b.time);
    } else {
        store.days.push(day);
        store
            .days
            .sort_by(|a, b| a.trading_date.cmp(&b.trading_date));
    }
    save_market_history(&store)?;
    Ok(true)
}

pub(crate) fn market_storage_info(current: &MarketAnalysisState) -> MarketStorageInfo {
    let store = load_market_history();
    let current_date = current
        .current
        .as_ref()
        .map(|snapshot| snapshot.trading_date.clone())
        .unwrap_or_default();
    let mut days = store
        .days
        .iter()
        .map(|day| MarketDaySummary {
            trading_date: day.trading_date.clone(),
            trend_points: day.history.len(),
            leader_label: day.snapshot.leader_label.clone(),
            status: day.snapshot.status.clone(),
            is_current: false,
        })
        .collect::<Vec<_>>();
    if let Some(snapshot) = current.current.as_ref() {
        if let Some(existing) = days
            .iter_mut()
            .find(|day| day.trading_date == snapshot.trading_date)
        {
            existing.trend_points += current.history.len();
            existing.leader_label = snapshot.leader_label.clone();
            existing.status = snapshot.status.clone();
            existing.is_current = true;
        } else {
            days.push(MarketDaySummary {
                trading_date: snapshot.trading_date.clone(),
                trend_points: current.history.len(),
                leader_label: snapshot.leader_label.clone(),
                status: snapshot.status.clone(),
                is_current: true,
            });
        }
    }
    days.sort_by(|a, b| b.trading_date.cmp(&a.trading_date));
    let size_bytes = [market_state_path(), market_history_path()]
        .iter()
        .filter_map(|path| fs::metadata(path).ok().map(|metadata| metadata.len()))
        .sum();
    MarketStorageInfo {
        total_days: days.len(),
        archived_days: store.days.len(),
        trend_points: days.iter().map(|day| day.trend_points).sum(),
        size_bytes,
        earliest_date: days
            .last()
            .map(|day| day.trading_date.clone())
            .unwrap_or_default(),
        latest_date: days
            .first()
            .map(|day| day.trading_date.clone())
            .unwrap_or_default(),
        current_date,
        days,
    }
}

pub(crate) fn delete_market_history_day(trading_date: &str) -> Result<bool, String> {
    let mut store = load_market_history();
    let before = store.days.len();
    store.days.retain(|day| day.trading_date != trading_date);
    if store.days.len() == before {
        return Ok(false);
    }
    save_market_history(&store)?;
    Ok(true)
}

pub(crate) fn clear_market_history() -> Result<(), String> {
    save_market_history(&MarketHistoryStore::default())
}

pub(crate) fn load_market_state() -> MarketAnalysisState {
    let state: MarketAnalysisState = fs::read_to_string(market_state_path())
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default();
    let incompatible = state.current.is_some()
        && (state.sample_version != crate::market_definition::active_definition_version()
            || state.algorithm_version != crate::market::ALGORITHM_VERSION);
    if incompatible {
        let _ = archive_market_day(&state);
    }
    let now = Local::now();
    normalize_market_state(
        state,
        &now.format("%Y-%m-%d").to_string(),
        now.weekday().number_from_monday() <= 5,
    )
}

pub(crate) fn save_market_state(state: &MarketAnalysisState) -> Result<(), String> {
    let path = market_state_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    atomic_write(
        &path,
        serde_json::to_string_pretty(state)
            .map_err(|error| error.to_string())?
            .as_bytes(),
    )
}

pub(crate) fn load_config() -> AppConfig {
    let path = config_path();
    if !path.exists() {
        let legacy = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(Path::to_path_buf))
            .map(|p| p.join("config.json"));
        if let Some(legacy) = legacy {
            if legacy.exists() {
                let _ = fs::create_dir_all(path.parent().unwrap_or(Path::new(".")));
                let _ = fs::copy(legacy, &path);
            }
        }
    }

    match fs::read_to_string(&path) {
        Ok(text) => match serde_json::from_str::<AppConfig>(&text) {
            Ok(config) => {
                let normalized = normalize_config(config);
                let _ = save_config_to(&path, &normalized);
                return normalized;
            }
            Err(_) => backup_invalid_config(&path),
        },
        Err(_) => {
            if path.exists() {
                backup_invalid_config(&path);
            }
        }
    }

    let config = default_config();
    let _ = save_config_to(&path, &config);
    config
}

pub(crate) fn save_config_to(path: &Path, config: &AppConfig) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let text = serde_json::to_string_pretty(&normalize_config(config.clone()))
        .map_err(|e| e.to_string())?;
    atomic_write(path, text.as_bytes())
}

fn empty_market_state() -> MarketAnalysisState {
    MarketAnalysisState {
        sample_version: crate::market_definition::active_definition_version(),
        algorithm_version: crate::market::ALGORITHM_VERSION.into(),
        ..Default::default()
    }
}

fn normalize_market_state(
    mut state: MarketAnalysisState,
    today: &str,
    is_weekday: bool,
) -> MarketAnalysisState {
    expire_market_state_if_needed(&mut state, today, is_weekday);
    state
}

pub(crate) fn expire_market_state_if_needed(
    state: &mut MarketAnalysisState,
    _today: &str,
    _is_weekday: bool,
) -> bool {
    let wrong_version = state.sample_version
        != crate::market_definition::active_definition_version()
        || state.algorithm_version != crate::market::ALGORITHM_VERSION;
    if wrong_version || (state.current.is_none() && !state.history.is_empty()) {
        *state = empty_market_state();
        true
    } else {
        false
    }
}

pub(crate) fn atomic_write(path: &Path, contents: &[u8]) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("data");
    let temporary = path.with_file_name(format!(".{name}.{}.{}.tmp", std::process::id(), stamp));
    let result = (|| {
        let mut file = File::create(&temporary).map_err(|error| error.to_string())?;
        file.write_all(contents)
            .map_err(|error| error.to_string())?;
        file.sync_all().map_err(|error| error.to_string())?;
        replace_file(&temporary, path).map_err(|error| error.to_string())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

#[cfg(windows)]
fn replace_file(source: &Path, target: &Path) -> std::io::Result<()> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::{
        MoveFileExW, MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH,
    };

    let source = source
        .as_os_str()
        .encode_wide()
        .chain(Some(0))
        .collect::<Vec<_>>();
    let target = target
        .as_os_str()
        .encode_wide()
        .chain(Some(0))
        .collect::<Vec<_>>();
    let moved = unsafe {
        MoveFileExW(
            source.as_ptr(),
            target.as_ptr(),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
    };
    if moved == 0 {
        Err(std::io::Error::last_os_error())
    } else {
        Ok(())
    }
}

#[cfg(not(windows))]
fn replace_file(source: &Path, target: &Path) -> std::io::Result<()> {
    fs::rename(source, target)
}

pub(crate) fn normalize_config(mut config: AppConfig) -> AppConfig {
    let original_schema_version = config.schema_version;
    migrate_config(&mut config, original_schema_version);
    config.schema_version = CURRENT_CONFIG_SCHEMA_VERSION;

    let mut tooltip_selected = false;
    for stock in &mut config.stocks {
        stock.holdings = normalize_holding(stock.holdings);
        stock.cost_price = normalize_cost_price(stock.cost_price);
        if stock.show_in_tooltip {
            if tooltip_selected {
                stock.show_in_tooltip = false;
            } else {
                tooltip_selected = true;
            }
        }
    }
    if !tooltip_selected {
        if let Some(stock) = config.stocks.first_mut() {
            stock.show_in_tooltip = true;
        }
    }
    if config.display_fields.is_empty() {
        config.display_fields = default_display_fields();
    }
    if config.tooltip_fields.is_empty() {
        config.tooltip_fields = default_tooltip_fields();
    }
    config
        .display_fields
        .retain(|field| is_supported_field(field) && field != "name" && field != "code");
    if config.display_fields.is_empty() {
        config.display_fields = default_display_fields();
    }
    config
        .tooltip_fields
        .retain(|field| is_supported_field(field));
    if config.tooltip_fields.is_empty() {
        config.tooltip_fields = default_tooltip_fields();
    }
    if config.theme.is_empty() {
        config.theme = default_theme();
    }
    config.background_refresh_interval_ms =
        normalize_background_refresh_interval_ms(config.background_refresh_interval_ms);
    config.popup.auto_hide_ms = normalize_auto_hide_ms(config.popup.auto_hide_ms);
    config.market_analysis.refresh_minutes = match config.market_analysis.refresh_minutes {
        5 | 15 | 30 => config.market_analysis.refresh_minutes,
        _ => 15,
    };
    normalize_external_data_sources(&mut config);
    normalize_data_source_order(&mut config);
    config
}

fn normalize_external_data_sources(config: &mut AppConfig) {
    let mut ids = HashSet::new();
    for (index, source) in config.external_data_sources.iter_mut().enumerate() {
        source.provider = source.provider.trim().to_ascii_lowercase();
        if source.provider.is_empty() {
            source.provider = "futu_opend".into();
        }
        source.name = source.name.trim().to_string();
        if source.name.is_empty() {
            source.name = "富途 OpenD".into();
        }
        source.host = source.host.trim().to_string();
        if source.host.is_empty() {
            source.host = "127.0.0.1".into();
        }
        if source.port == 0 {
            source.port = 32179;
        }

        let base_id = if source.id.trim().is_empty() {
            format!("{}-{}", source.provider, index + 1)
        } else {
            source.id.trim().to_string()
        };
        let mut id = base_id.clone();
        let mut suffix = 2;
        while !ids.insert(id.clone()) {
            id = format!("{base_id}-{suffix}");
            suffix += 1;
        }
        source.id = id;
    }
}

fn normalize_data_source_order(config: &mut AppConfig) {
    let valid_external = config
        .external_data_sources
        .iter()
        .map(|source| source.id.as_str())
        .collect::<HashSet<_>>();
    let mut seen = HashSet::new();
    config.data_source_order.retain(|key| {
        let valid = matches!(key.as_str(), "eastmoney" | "tencent")
            || valid_external.contains(key.as_str());
        valid && seen.insert(key.clone())
    });
    for source in &config.external_data_sources {
        if seen.insert(source.id.clone()) {
            config.data_source_order.insert(0, source.id.clone());
        }
    }
    for key in ["eastmoney", "tencent"] {
        if seen.insert(key.into()) {
            config.data_source_order.push(key.into());
        }
    }
}

fn migrate_config(config: &mut AppConfig, from_version: u32) {
    if from_version < 2 {
        // v2 makes tray tooltip stock selection single-choice. The normalizer below
        // enforces the final invariant while preserving the first existing choice.
    }
    if from_version < 3 {
        // v3 adds background quote refresh. Serde default fills the field for old configs.
    }
    if from_version < 4
        && !config
            .tooltip_fields
            .iter()
            .any(|field| field == "position_pnl")
    {
        config.tooltip_fields.push("position_pnl".into());
    }
    if from_version < 5 {
        // v5 adds the independent market-analysis schedule; serde supplies defaults.
    }
    if from_version < 6 {
        // v6 adds extensible external data-source configuration.
    }
}

fn backup_invalid_config(path: &Path) {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0);
    let backup_path = path.with_file_name(format!("config.invalid.{timestamp}.json"));
    let _ = fs::copy(path, backup_path);
}

pub(crate) fn normalize_holding(value: f32) -> f32 {
    if value.is_finite() {
        ((value.max(0.0) / 100.0).round() * 100.0).max(0.0)
    } else {
        0.0
    }
}

pub(crate) fn normalize_cost_price(value: f32) -> f32 {
    if value.is_finite() {
        round3(value)
    } else {
        0.0
    }
}

fn normalize_background_refresh_interval_ms(value: u32) -> u32 {
    if value == 0 {
        0
    } else {
        value.clamp(1_000, 600_000)
    }
}

fn normalize_auto_hide_ms(value: u32) -> u32 {
    if value == 0 {
        0
    } else {
        let clamped = value.clamp(100, 30_000);
        ((clamped + 50) / 100) * 100
    }
}

pub(crate) fn is_supported_field(field: &str) -> bool {
    matches!(
        field,
        "name"
            | "code"
            | "price"
            | "prev_close"
            | "open"
            | "high"
            | "low"
            | "change"
            | "change_percent"
            | "volume"
            | "amount"
            | "volume_ratio"
            | "turnover"
            | "holdings"
            | "cost_price"
            | "daily_pnl"
            | "daily_pnl_percent"
            | "position_pnl"
            | "position_pnl_percent"
    )
}

pub(crate) fn default_config() -> AppConfig {
    AppConfig {
        schema_version: CURRENT_CONFIG_SCHEMA_VERSION,
        stocks: vec![
            StockEntry {
                code: "sh600519".into(),
                name: "贵州茅台".into(),
                holdings: 100.0,
                cost_price: 0.0,
                show_in_popup: true,
                show_in_tooltip: true,
            },
            StockEntry {
                code: "sz000858".into(),
                name: "五粮液".into(),
                holdings: 500.0,
                cost_price: 0.0,
                show_in_popup: true,
                show_in_tooltip: false,
            },
            StockEntry {
                code: "sh600036".into(),
                name: "招商银行".into(),
                holdings: 1000.0,
                cost_price: 0.0,
                show_in_popup: true,
                show_in_tooltip: false,
            },
            StockEntry {
                code: "sz300750".into(),
                name: "宁德时代".into(),
                holdings: 200.0,
                cost_price: 0.0,
                show_in_popup: true,
                show_in_tooltip: false,
            },
        ],
        theme: default_theme(),
        show_daily_summary: true,
        background_refresh_interval_ms: default_background_refresh_interval_ms(),
        market_analysis: MarketAnalysisConfig::default(),
        popup: PopupConfig::default(),
        appearance: AppearanceConfig::default(),
        external_data_sources: Vec::new(),
        data_source_order: crate::models::default_data_source_order(),
        display_fields: default_display_fields(),
        tooltip_fields: default_tooltip_fields(),
    }
}

fn round3(value: f32) -> f32 {
    (value * 1000.0).round() / 1000.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{MarketEvidence, MarketSnapshot};

    fn market_state(date: &str) -> MarketAnalysisState {
        MarketAnalysisState {
            current: Some(MarketSnapshot {
                trading_date: date.into(),
                ..Default::default()
            }),
            history: vec![MarketEvidence::default()],
            sample_version: crate::market_definition::active_definition_version(),
            algorithm_version: crate::market::ALGORITHM_VERSION.into(),
            ..Default::default()
        }
    }

    #[test]
    fn market_state_expires_on_version_but_waits_for_the_next_valid_trading_day() {
        let mut wrong_version = market_state("2026-07-10");
        wrong_version.algorithm_version = "old".into();
        assert!(normalize_market_state(wrong_version, "2026-07-10", true)
            .current
            .is_none());
        assert!(
            normalize_market_state(market_state("2026-07-10"), "2026-07-13", true)
                .current
                .is_some()
        );
        assert!(
            normalize_market_state(market_state("2026-07-10"), "2026-07-12", false)
                .current
                .is_some()
        );
    }

    #[test]
    fn atomic_write_replaces_existing_content() {
        let path = std::env::temp_dir().join(format!(
            "stocktray-atomic-{}-{}.json",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        atomic_write(&path, b"old").unwrap();
        atomic_write(&path, b"new").unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"new");
        let _ = fs::remove_file(path);
    }

    #[test]
    fn config_normalization_preserves_ui_contracts() {
        let mut config = default_config();
        for stock in &mut config.stocks {
            stock.show_in_tooltip = true;
        }
        config.display_fields = vec!["name".into(), "unknown".into()];
        config.tooltip_fields.clear();
        config.background_refresh_interval_ms = 0;
        config.popup.auto_hide_ms = 42;
        config.market_analysis.refresh_minutes = 7;
        config.external_data_sources = vec![
            crate::models::ExternalDataSourceConfig {
                id: "futu-main".into(),
                provider: " FUTU_OPEND ".into(),
                name: " ".into(),
                host: " 550W ".into(),
                port: 0,
                enabled: true,
            },
            crate::models::ExternalDataSourceConfig {
                id: "futu-main".into(),
                provider: "futu_opend".into(),
                name: "备用 OpenD".into(),
                host: "127.0.0.1".into(),
                port: 11111,
                enabled: false,
            },
        ];
        config.data_source_order = vec![
            "tencent".into(),
            "futu-main".into(),
            "unknown".into(),
            "eastmoney".into(),
            "tencent".into(),
        ];

        let normalized = normalize_config(config);
        assert_eq!(
            normalized
                .stocks
                .iter()
                .filter(|stock| stock.show_in_tooltip)
                .count(),
            1
        );
        assert_eq!(normalized.display_fields, default_display_fields());
        assert_eq!(normalized.tooltip_fields, default_tooltip_fields());
        assert_eq!(normalized.background_refresh_interval_ms, 0);
        assert_eq!(normalized.popup.auto_hide_ms, 100);
        assert_eq!(normalized.market_analysis.refresh_minutes, 15);
        assert_eq!(normalized.external_data_sources[0].provider, "futu_opend");
        assert_eq!(normalized.external_data_sources[0].name, "富途 OpenD");
        assert_eq!(normalized.external_data_sources[0].host, "550W");
        assert_eq!(normalized.external_data_sources[0].port, 32179);
        assert_eq!(normalized.external_data_sources[1].id, "futu-main-2");
        assert_eq!(
            normalized.data_source_order,
            vec!["futu-main-2", "tencent", "futu-main", "eastmoney"]
        );
        assert_eq!(normalize_auto_hide_ms(151), 200);
        assert_eq!(normalize_auto_hide_ms(30_001), 30_000);
    }
}
