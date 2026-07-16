use std::{collections::HashSet, path::PathBuf, time::Duration};

use chrono::{Local, NaiveDate};
use minisign_verify::{PublicKey, Signature};
use serde::{Deserialize, Serialize};

const DEFINITION_SCHEMA_VERSION: u32 = 1;
const REMOTE_BASE_URL: &str =
    "https://github.com/keyu-0915/StockTray/releases/download/market-data";
const REMOTE_POINTER_URL: &str =
    "https://github.com/keyu-0915/StockTray/releases/download/market-data/stable.json";
const MARKET_DEFINITION_PUBLIC_KEY: &str =
    "RWSUz1mQgbQGjDmp62//6wyhDlE6bi/t03NtjHNIJTBPBkPIW52eYnJ6";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct DefinitionLimits {
    pub(crate) min_total: usize,
    pub(crate) max_total: usize,
    pub(crate) min_per_style: usize,
    pub(crate) max_per_style: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct BoardDefinition {
    pub(crate) name: String,
    pub(crate) code: String,
    #[serde(default = "default_sample_weight")]
    pub(crate) sample_weight: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct StyleDefinition {
    pub(crate) id: String,
    pub(crate) label: String,
    pub(crate) subtitle: String,
    pub(crate) boards: Vec<BoardDefinition>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct FallbackGroup {
    pub(crate) style: String,
    pub(crate) subsector: String,
    pub(crate) codes: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct MarketDefinition {
    pub(crate) schema_version: u32,
    pub(crate) definition_version: String,
    pub(crate) min_app_version: String,
    #[serde(default)]
    pub(crate) effective_from: String,
    pub(crate) member_refresh_days: i64,
    pub(crate) limits: DefinitionLimits,
    pub(crate) styles: Vec<StyleDefinition>,
    pub(crate) fallback_groups: Vec<FallbackGroup>,
}

#[derive(Debug, Deserialize)]
struct DefinitionPointer {
    schema_version: u32,
    definition_version: String,
    definition_file: String,
    signature_file: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct CachedDefinition {
    raw: String,
    signature: String,
    activated_on: String,
}

#[derive(Debug, Clone)]
pub(crate) struct SignedDefinition {
    pub(crate) definition: MarketDefinition,
    raw: String,
    signature: String,
}

fn default_sample_weight() -> u8 {
    1
}

impl MarketDefinition {
    pub(crate) fn embedded() -> Self {
        parse_definition(include_str!(
            "../../docs/market/definitions/2026.07-v5.json"
        ))
        .expect("embedded market definition must be valid")
    }

    pub(crate) fn style(&self, id: &str) -> Option<&StyleDefinition> {
        self.styles.iter().find(|style| style.id == id)
    }

    pub(crate) fn boards(&self) -> impl Iterator<Item = (&str, &BoardDefinition)> {
        self.styles.iter().flat_map(|style| {
            style
                .boards
                .iter()
                .map(move |board| (style.id.as_str(), board))
        })
    }

    pub(crate) fn index_secids(&self) -> Vec<String> {
        self.boards()
            .map(|(_, board)| format!("90.{}", board.code))
            .chain(
                crate::market::BROAD_INDICES
                    .iter()
                    .map(|code| (*code).to_string()),
            )
            .collect()
    }

    pub(crate) fn is_effective(&self, today: NaiveDate) -> bool {
        self.effective_from.is_empty()
            || NaiveDate::parse_from_str(&self.effective_from, "%Y-%m-%d")
                .map(|date| date <= today)
                .unwrap_or(false)
    }
}

pub(crate) fn active_definition() -> (MarketDefinition, String) {
    load_cached_definition(active_definition_path())
        .map(|signed| (signed.definition, "remote_signed".into()))
        .unwrap_or_else(|| (MarketDefinition::embedded(), "embedded".into()))
}

pub(crate) fn active_definition_version() -> String {
    active_definition().0.definition_version
}

pub(crate) fn load_pending_definition() -> Option<SignedDefinition> {
    load_cached_definition(pending_definition_path())
}

pub(crate) fn save_pending_definition(signed: &SignedDefinition) -> Result<(), String> {
    save_cached_definition(pending_definition_path(), signed)
}

pub(crate) fn activate_definition(signed: &SignedDefinition) -> Result<(), String> {
    let active = active_definition_path();
    if active.exists() {
        let previous = previous_definition_path();
        if let Ok(contents) = std::fs::read(&active) {
            crate::config::atomic_write(&previous, &contents)?;
        }
    }
    save_cached_definition(active, signed)?;
    let pending = pending_definition_path();
    if pending.exists() {
        let _ = std::fs::remove_file(pending);
    }
    Ok(())
}

pub(crate) async fn fetch_remote_definition(
    current_version: &str,
) -> Result<Option<SignedDefinition>, String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .user_agent("Mozilla/5.0 StockTray market-definition")
        .build()
        .map_err(|error| error.to_string())?;
    let pointer = client
        .get(REMOTE_POINTER_URL)
        .header(reqwest::header::CACHE_CONTROL, "no-cache")
        .send()
        .await
        .map_err(|error| error.to_string())?
        .error_for_status()
        .map_err(|error| error.to_string())?
        .json::<DefinitionPointer>()
        .await
        .map_err(|error| error.to_string())?;
    validate_pointer(&pointer)?;
    if pointer.definition_version == current_version {
        return Ok(None);
    }
    let definition_url = format!("{REMOTE_BASE_URL}/{}", pointer.definition_file);
    let signature_url = format!("{REMOTE_BASE_URL}/{}", pointer.signature_file);
    let raw = client
        .get(definition_url)
        .send()
        .await
        .map_err(|error| error.to_string())?
        .error_for_status()
        .map_err(|error| error.to_string())?
        .text()
        .await
        .map_err(|error| error.to_string())?;
    let signature = client
        .get(signature_url)
        .send()
        .await
        .map_err(|error| error.to_string())?
        .error_for_status()
        .map_err(|error| error.to_string())?
        .text()
        .await
        .map_err(|error| error.to_string())?;
    verify_signature(raw.as_bytes(), &signature)?;
    let definition = parse_definition(&raw)?;
    if definition.definition_version != pointer.definition_version {
        return Err("远程市场定义版本与指针不一致".into());
    }
    ensure_minimum_app_version(&definition.min_app_version)?;
    Ok(Some(SignedDefinition {
        definition,
        raw,
        signature,
    }))
}

fn validate_pointer(pointer: &DefinitionPointer) -> Result<(), String> {
    if pointer.schema_version != DEFINITION_SCHEMA_VERSION
        || !safe_asset_name(&pointer.definition_file, ".json")
        || !safe_asset_name(&pointer.signature_file, ".sig")
        || pointer.signature_file != format!("{}.sig", pointer.definition_file)
        || pointer.definition_version.trim().is_empty()
    {
        return Err("远程市场定义指针无效".into());
    }
    Ok(())
}

fn safe_asset_name(value: &str, suffix: &str) -> bool {
    !value.is_empty()
        && value.ends_with(suffix)
        && value
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || ".-_".contains(character))
}

fn ensure_minimum_app_version(required: &str) -> Result<(), String> {
    let current =
        semver::Version::parse(env!("CARGO_PKG_VERSION")).map_err(|error| error.to_string())?;
    let required = semver::Version::parse(required).map_err(|error| error.to_string())?;
    if current < required {
        return Err(format!("市场定义需要客户端 {required} 或更高版本"));
    }
    Ok(())
}

fn verify_signature(raw: &[u8], signature: &str) -> Result<(), String> {
    let public_key =
        PublicKey::from_base64(MARKET_DEFINITION_PUBLIC_KEY).map_err(|error| error.to_string())?;
    let signature = Signature::decode(signature).map_err(|error| error.to_string())?;
    public_key
        .verify(raw, &signature, false)
        .map_err(|error| format!("市场定义签名验证失败: {error}"))
}

fn parse_definition(raw: &str) -> Result<MarketDefinition, String> {
    let definition = serde_json::from_str::<MarketDefinition>(raw)
        .map_err(|error| format!("市场定义格式错误: {error}"))?;
    validate_definition(&definition)?;
    Ok(definition)
}

fn validate_definition(definition: &MarketDefinition) -> Result<(), String> {
    if definition.schema_version != DEFINITION_SCHEMA_VERSION
        || definition.definition_version.trim().is_empty()
        || semver::Version::parse(&definition.min_app_version).is_err()
        || (!definition.effective_from.is_empty()
            && NaiveDate::parse_from_str(&definition.effective_from, "%Y-%m-%d").is_err())
        || definition.member_refresh_days < 1
        || definition.member_refresh_days > 30
        || definition.limits.min_total == 0
        || definition.limits.min_total > definition.limits.max_total
        || definition.limits.min_per_style == 0
        || definition.limits.min_per_style > definition.limits.max_per_style
    {
        return Err("市场定义版本或数量限制无效".into());
    }
    let required_styles = ["young", "middle", "old"];
    if definition.styles.len() != required_styles.len()
        || required_styles
            .iter()
            .any(|id| definition.styles.iter().all(|style| style.id != *id))
    {
        return Err("市场定义必须完整包含小登、中登和老登".into());
    }
    let mut board_codes = HashSet::new();
    for style in &definition.styles {
        if style.label.trim().is_empty()
            || style.subtitle.trim().is_empty()
            || style.boards.is_empty()
            || style.boards.len() > 16
        {
            return Err(format!("{} 的板块定义无效", style.id));
        }
        for board in &style.boards {
            if board.name.trim().is_empty()
                || !valid_board_code(&board.code)
                || !(1..=10).contains(&board.sample_weight)
                || !board_codes.insert(board.code.clone())
            {
                return Err(format!("板块 {} 的定义无效或重复", board.code));
            }
        }
    }
    let style_ids = definition
        .styles
        .iter()
        .map(|style| style.id.as_str())
        .collect::<HashSet<_>>();
    let mut codes = HashSet::new();
    let mut style_counts = std::collections::HashMap::new();
    for group in &definition.fallback_groups {
        if !style_ids.contains(group.style.as_str()) || group.subsector.trim().is_empty() {
            return Err("离线样本包含未知风格或空分类".into());
        }
        for code in group.codes.split(',') {
            if !valid_stock_code(code) || !codes.insert(code) {
                return Err(format!("离线样本代码无效或重复: {code}"));
            }
            *style_counts.entry(group.style.as_str()).or_insert(0usize) += 1;
        }
    }
    if !(definition.limits.min_total..=definition.limits.max_total).contains(&codes.len())
        || required_styles.iter().any(|style| {
            style_counts.get(style).copied().unwrap_or(0) < definition.limits.min_per_style
        })
    {
        return Err("离线样本未达到远程定义的覆盖门槛".into());
    }
    Ok(())
}

fn valid_board_code(code: &str) -> bool {
    code.len() == 6
        && code.starts_with("BK")
        && code[2..]
            .chars()
            .all(|character| character.is_ascii_digit())
}

fn valid_stock_code(code: &str) -> bool {
    code.len() == 8
        && matches!(&code[..2], "sh" | "sz")
        && code[2..]
            .chars()
            .all(|character| character.is_ascii_digit())
}

fn load_cached_definition(path: PathBuf) -> Option<SignedDefinition> {
    let cache = std::fs::read_to_string(path)
        .ok()
        .and_then(|raw| serde_json::from_str::<CachedDefinition>(&raw).ok())?;
    verify_signature(cache.raw.as_bytes(), &cache.signature).ok()?;
    let definition = parse_definition(&cache.raw).ok()?;
    ensure_minimum_app_version(&definition.min_app_version).ok()?;
    Some(SignedDefinition {
        definition,
        raw: cache.raw,
        signature: cache.signature,
    })
}

fn save_cached_definition(path: PathBuf, signed: &SignedDefinition) -> Result<(), String> {
    let cache = CachedDefinition {
        raw: signed.raw.clone(),
        signature: signed.signature.clone(),
        activated_on: Local::now().format("%Y-%m-%d").to_string(),
    };
    let raw = serde_json::to_string_pretty(&cache).map_err(|error| error.to_string())?;
    crate::config::atomic_write(&path, raw.as_bytes())
}

fn active_definition_path() -> PathBuf {
    crate::config::config_path().with_file_name("market-definition.json")
}

fn previous_definition_path() -> PathBuf {
    crate::config::config_path().with_file_name("market-definition.previous.json")
}

fn pending_definition_path() -> PathBuf {
    crate::config::config_path().with_file_name("market-definition.pending.json")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_definition_is_complete_and_valid() {
        let definition = MarketDefinition::embedded();
        assert_eq!(definition.schema_version, 1);
        assert_eq!(definition.styles.len(), 3);
        assert!(definition
            .style("middle")
            .unwrap()
            .boards
            .iter()
            .any(|board| { board.name == "创新药" && board.code == "BK1106" }));
    }

    #[test]
    fn stable_pointer_targets_a_valid_definition() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../docs/market");
        let pointer = std::fs::read_to_string(root.join("stable.json"))
            .ok()
            .and_then(|raw| serde_json::from_str::<DefinitionPointer>(&raw).ok())
            .expect("stable market pointer must be valid JSON");
        validate_pointer(&pointer).expect("stable market pointer must be valid");
        let raw = std::fs::read_to_string(root.join("definitions").join(pointer.definition_file))
            .expect("stable market definition must exist");
        let definition = parse_definition(&raw).expect("stable market definition must be valid");
        assert_eq!(definition.definition_version, pointer.definition_version);
    }

    #[test]
    fn asset_names_cannot_escape_the_release_directory() {
        assert!(safe_asset_name("2026.07-v5.json", ".json"));
        assert!(!safe_asset_name("../definition.json", ".json"));
        assert!(!safe_asset_name("https://example.com/a.json", ".json"));
    }

    #[test]
    fn invalid_remote_board_or_weight_is_rejected() {
        let mut definition = MarketDefinition::embedded();
        definition.styles[0].boards[0].code = "https://bad".into();
        assert!(validate_definition(&definition).is_err());

        let mut definition = MarketDefinition::embedded();
        definition.styles[0].boards[0].sample_weight = 0;
        assert!(validate_definition(&definition).is_err());
    }
}
