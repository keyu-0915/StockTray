use std::time::Duration;

use encoding_rs::GBK;
use regex::Regex;
use reqwest::Client;

use crate::models::StockData;

const BATCH_SIZE: usize = 80;
const BOARD_PAGE_SIZE: usize = 100;
const MAX_BOARD_PAGES: usize = 20;

#[derive(Debug, Clone, Default)]
pub(crate) struct QuoteFetchResult {
    pub(crate) quotes: Vec<StockData>,
    pub(crate) index_quotes: Vec<StockData>,
    pub(crate) index_error: String,
    pub(crate) primary_count: usize,
    pub(crate) fallback_count: usize,
}

pub(crate) async fn fetch_index_quotes(secids: &[String]) -> Result<Vec<StockData>, String> {
    if secids.is_empty() {
        return Ok(Vec::new());
    }
    let client = http_client()?;
    let url = format!("https://push2.eastmoney.com/api/qt/ulist.np/get?fltt=2&fields=f2,f3,f4,f12,f14,f17,f18,f124&secids={}&_={}", secids.join(","), chrono::Utc::now().timestamp_millis());
    let json: serde_json::Value = client
        .get(url)
        .send()
        .await
        .map_err(|error| error.to_string())?
        .json()
        .await
        .map_err(|error| error.to_string())?;
    let list = json["data"]["diff"]
        .as_array()
        .ok_or("index quote has no data")?;
    Ok(list
        .iter()
        .filter_map(|item| {
            let code = item["f12"].as_str()?;
            let price = json_f32(item, "f2");
            let prev_close = json_f32(item, "f18");
            Some(StockData {
                code: format!("em:{code}"),
                name: item["f14"].as_str().unwrap_or(code).into(),
                price,
                prev_close,
                open: json_f32(item, "f17"),
                change: json_f32(item, "f4"),
                change_percent: json_f32(item, "f3"),
                date: quote_datetime(item, "%Y-%m-%d"),
                time: quote_datetime(item, "%H:%M:%S"),
                source: "eastmoney".into(),
                ..Default::default()
            })
        })
        .collect())
}

#[derive(Debug, Clone)]
pub(crate) struct BoardMember {
    pub(crate) code: String,
    pub(crate) name: String,
}

pub(crate) async fn fetch_quotes(codes: &[String]) -> Result<Vec<StockData>, String> {
    Ok(fetch_quotes_detailed(codes).await?.quotes)
}

pub(crate) async fn fetch_quotes_detailed(codes: &[String]) -> Result<QuoteFetchResult, String> {
    if codes.is_empty() {
        return Ok(QuoteFetchResult::default());
    }
    let client = http_client()?;
    let mut result = QuoteFetchResult::default();
    let mut primary_available = true;
    for batch in codes.chunks(BATCH_SIZE) {
        let primary = if primary_available {
            match fetch_eastmoney(&client, batch).await {
                Ok(quotes) => quotes,
                Err(_) => {
                    primary_available = false;
                    Vec::new()
                }
            }
        } else {
            Vec::new()
        };
        result.primary_count += primary.iter().filter(|q| q.error.is_empty()).count();
        let missing = batch
            .iter()
            .filter(|code| {
                !primary
                    .iter()
                    .any(|quote| quote.code.as_str() == code.as_str() && quote.error.is_empty())
            })
            .cloned()
            .collect::<Vec<_>>();
        result.quotes.extend(primary);
        if !missing.is_empty() {
            let fallback = fetch_tencent(&client, &missing).await.unwrap_or_else(|_| {
                missing
                    .iter()
                    .map(|code| StockData {
                        code: code.clone(),
                        error: "no_data".into(),
                        ..Default::default()
                    })
                    .collect()
            });
            result.fallback_count += fallback.iter().filter(|q| q.error.is_empty()).count();
            result.quotes.extend(fallback);
        }
    }
    Ok(result)
}

pub(crate) async fn fetch_board_members(board: &str) -> Result<Vec<BoardMember>, String> {
    let client = Client::builder()
        .timeout(Duration::from_secs(4))
        .user_agent("Mozilla/5.0 StockTray/0.2")
        .build()
        .map_err(|error| error.to_string())?;
    let mut members = Vec::new();
    for page in 1..=MAX_BOARD_PAGES {
        let (mut batch, total) = fetch_board_page(&client, board, page).await?;
        let batch_len = batch.len();
        members.append(&mut batch);
        if members.len() >= total || batch_len < BOARD_PAGE_SIZE {
            members.sort_by(|a, b| a.code.cmp(&b.code));
            members.dedup_by(|a, b| a.code == b.code);
            return Ok(members);
        }
    }
    Err(format!("{board}: board constituents exceeded page limit"))
}

async fn fetch_board_page(
    client: &Client,
    board: &str,
    page: usize,
) -> Result<(Vec<BoardMember>, usize), String> {
    let mut last_error = String::new();
    for host in ["29", "17"] {
        let url = format!("https://{host}.push2.eastmoney.com/api/qt/clist/get?pn={page}&pz={BOARD_PAGE_SIZE}&po=0&np=1&ut=bd1d9ddb04089700cf9c27f6f7426281&fltt=2&invt=2&fid=f12&fs=b:{board}%20f:!50&fields=f12,f13,f14&_={}", chrono::Utc::now().timestamp_millis());
        match client.get(url).send().await {
            Ok(result) => match result.json::<serde_json::Value>().await {
                Ok(json) => match parse_board_page(&json) {
                    Ok(parsed) => return Ok(parsed),
                    Err(error) => last_error = error,
                },
                Err(error) => last_error = error.to_string(),
            },
            Err(error) => last_error = error.to_string(),
        }
    }
    Err(format!("{board}: {last_error}"))
}

fn parse_board_page(json: &serde_json::Value) -> Result<(Vec<BoardMember>, usize), String> {
    let list = json["data"]["diff"]
        .as_array()
        .ok_or_else(|| "board has no constituents".to_string())?;
    let members = list
        .iter()
        .filter_map(|item| {
            let number = item["f12"].as_str()?;
            let market = item["f13"].as_i64()?;
            let prefix = if market == 1 { "sh" } else { "sz" };
            Some(BoardMember {
                code: format!("{prefix}{number}"),
                name: item["f14"].as_str().unwrap_or(number).to_string(),
            })
        })
        .collect::<Vec<_>>();
    let total = json["data"]["total"]
        .as_u64()
        .map(|value| value as usize)
        .unwrap_or(members.len());
    Ok((members, total))
}

pub(crate) async fn fetch_stock_name(code: &str) -> Result<String, String> {
    let client = http_client()?;
    let secid = eastmoney_secid(code);
    let url = format!(
        "https://push2.eastmoney.com/api/qt/stock/get?secid={secid}&fields=f57&_={}",
        chrono::Utc::now().timestamp_millis()
    );
    let json: serde_json::Value = client
        .get(url)
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())?;
    Ok(json["data"]["f57"].as_str().unwrap_or(code).to_string())
}

fn http_client() -> Result<Client, String> {
    Client::builder()
        .timeout(Duration::from_secs(8))
        .user_agent("StockTray/0.2")
        .build()
        .map_err(|e| e.to_string())
}

async fn fetch_eastmoney(client: &Client, codes: &[String]) -> Result<Vec<StockData>, String> {
    let secids = codes.iter().map(|c| eastmoney_secid(c)).collect::<Vec<_>>();
    let url = format!(
        "https://push2.eastmoney.com/api/qt/ulist.np/get?fltt=2&fields=f2,f3,f4,f5,f6,f8,f10,f12,f14,f15,f16,f17,f18,f21,f26,f124&secids={}&_={}",
        secids.join(","),
        chrono::Utc::now().timestamp_millis()
    );
    let json: serde_json::Value = client
        .get(url)
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())?;
    let list = json["data"]["diff"].as_array().ok_or("eastmoney no data")?;
    let mut result = Vec::new();
    for secid in secids {
        let code_num = secid.split('.').next_back().unwrap_or("");
        let full = format!(
            "{}{}",
            if secid.starts_with("1.") { "sh" } else { "sz" },
            code_num
        );
        if let Some(item) = list.iter().find(|v| v["f12"].as_str() == Some(code_num)) {
            let price = json_f32(item, "f2");
            let prev_close = json_f32(item, "f18");
            let mut change = json_f32(item, "f4");
            let mut change_percent = json_f32(item, "f3");
            if change == 0.0 && prev_close > 0.0 && price > 0.0 {
                change = round3(price - prev_close);
            }
            if change_percent == 0.0 && prev_close > 0.0 && price > 0.0 {
                change_percent = round2((price - prev_close) / prev_close * 100.0);
            }
            let name = item["f14"].as_str().unwrap_or("").to_string();
            let (upper_limit, lower_limit) = theoretical_price_limits(&full, &name, prev_close);
            result.push(StockData {
                code: full,
                name,
                price,
                prev_close,
                open: json_f32(item, "f17"),
                high: json_f32(item, "f15"),
                low: json_f32(item, "f16"),
                volume: json_f32(item, "f5"),
                amount: json_f32(item, "f6"),
                volume_ratio: json_f32(item, "f10"),
                change,
                change_percent,
                turnover: json_f32(item, "f8"),
                float_market_cap: json_f64(item, "f21"),
                upper_limit,
                lower_limit,
                listing_date: json_date(item, "f26"),
                source: "eastmoney".into(),
                date: quote_datetime(item, "%Y-%m-%d"),
                time: quote_datetime(item, "%H:%M:%S"),
                ..Default::default()
            });
        }
    }
    if result.is_empty() {
        Err("eastmoney empty".into())
    } else {
        Ok(result)
    }
}

fn theoretical_price_limits(code: &str, name: &str, prev_close: f32) -> (f32, f32) {
    if prev_close <= 0.0 || name.to_ascii_uppercase().contains("ST") {
        return (0.0, 0.0);
    }
    let number = code.get(2..).unwrap_or(code);
    let rate = if number.starts_with("300")
        || number.starts_with("301")
        || number.starts_with("688")
        || number.starts_with("689")
    {
        0.20
    } else {
        0.10
    };
    (
        round2(prev_close * (1.0 + rate)),
        round2(prev_close * (1.0 - rate)),
    )
}

async fn fetch_tencent(client: &Client, codes: &[String]) -> Result<Vec<StockData>, String> {
    let url = format!("https://qt.gtimg.cn/q={}", codes.join(","));
    let bytes = client
        .get(url)
        .header("Referer", "https://gu.qq.com/")
        .send()
        .await
        .map_err(|e| e.to_string())?
        .bytes()
        .await
        .map_err(|e| e.to_string())?;
    let (text, _, _) = GBK.decode(&bytes);
    let text = text.to_string();
    let mut result = Vec::new();
    for code in codes {
        let pattern = format!(r#"v_{}="([^"]*)""#, regex::escape(code));
        if let Some(caps) = Regex::new(&pattern)
            .map_err(|e| e.to_string())?
            .captures(&text)
        {
            result.push(parse_tencent_line(code, &caps[1]));
        } else {
            result.push(StockData {
                code: code.clone(),
                error: "no_data".into(),
                ..Default::default()
            });
        }
    }
    Ok(result)
}

fn parse_tencent_line(code: &str, line: &str) -> StockData {
    let parts = line.split('~').collect::<Vec<_>>();
    if parts.len() < 38 {
        return StockData {
            code: code.into(),
            error: "incomplete_data".into(),
            ..Default::default()
        };
    }
    let price = parse_f32(parts[3]);
    let prev_close = parse_f32(parts[4]);
    let change = if prev_close > 0.0 && price > 0.0 {
        round3(price - prev_close)
    } else {
        0.0
    };
    let change_percent = if prev_close > 0.0 && price > 0.0 {
        round2(change / prev_close * 100.0)
    } else {
        0.0
    };
    StockData {
        code: code.into(),
        name: parts[1].into(),
        price,
        prev_close,
        open: parse_f32(parts[5]),
        high: parse_f32(parts[33]),
        low: parse_f32(parts[34]),
        volume: parse_f32(parts[6]),
        amount: parse_f32(parts[37]) * 10_000.0,
        volume_ratio: parts.get(49).map_or(0.0, |value| parse_f32(value)),
        change,
        change_percent,
        turnover: parts.get(38).map_or(0.0, |value| parse_f32(value)),
        float_market_cap: parts
            .get(45)
            .map_or(0.0, |value| parse_f64(value) * 100_000_000.0),
        upper_limit: parts.get(47).map_or(0.0, |value| parse_f32(value)),
        lower_limit: parts.get(48).map_or(0.0, |value| parse_f32(value)),
        date: parse_tencent_timestamp(parts[30], "%Y-%m-%d"),
        time: parse_tencent_timestamp(parts[30], "%H:%M:%S"),
        source: "tencent".into(),
        ..Default::default()
    }
}

pub(crate) fn normalize_code(input: &str) -> Option<String> {
    let mut code = input.trim().to_lowercase();
    if code.is_empty() {
        return None;
    }
    if code.starts_with("sh") || code.starts_with("sz") {
        return (code.len() == 8 && code[2..].chars().all(|c| c.is_ascii_digit())).then_some(code);
    }
    while code.len() < 6 {
        code = format!("0{code}");
    }
    if code.len() != 6 || !code.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    if code == "000000" {
        return None;
    }
    if code.starts_with(['6', '5', '9']) {
        Some(format!("sh{code}"))
    } else if code.starts_with(['0', '3', '2', '1']) {
        Some(format!("sz{code}"))
    } else {
        None
    }
}

fn eastmoney_secid(code: &str) -> String {
    if let Some(code_num) = code.strip_prefix("sh") {
        format!("1.{code_num}")
    } else if let Some(code_num) = code.strip_prefix("sz") {
        format!("0.{code_num}")
    } else {
        format!("0.{}", &code[2..])
    }
}

fn json_f32(value: &serde_json::Value, key: &str) -> f32 {
    value[key]
        .as_f64()
        .map(|v| v as f32)
        .unwrap_or_else(|| parse_f32(value[key].as_str().unwrap_or("0")))
}

fn json_f64(value: &serde_json::Value, key: &str) -> f64 {
    value[key]
        .as_f64()
        .unwrap_or_else(|| value[key].as_str().unwrap_or("0").parse().unwrap_or(0.0))
}

fn quote_datetime(value: &serde_json::Value, format: &str) -> String {
    value["f124"]
        .as_i64()
        .and_then(|seconds| chrono::DateTime::from_timestamp(seconds, 0))
        .map(|date| {
            date.with_timezone(&chrono::Local)
                .format(format)
                .to_string()
        })
        .unwrap_or_default()
}

fn json_date(value: &serde_json::Value, key: &str) -> String {
    let raw = value[key]
        .as_i64()
        .map(|number| number.to_string())
        .or_else(|| value[key].as_str().map(str::to_string))
        .unwrap_or_default();
    chrono::NaiveDate::parse_from_str(&raw, "%Y%m%d")
        .map(|date| date.format("%Y-%m-%d").to_string())
        .unwrap_or_default()
}

fn parse_tencent_timestamp(value: &str, format: &str) -> String {
    chrono::NaiveDateTime::parse_from_str(value, "%Y%m%d%H%M%S")
        .map(|date| date.format(format).to_string())
        .unwrap_or_default()
}

fn parse_f32(value: &str) -> f32 {
    value.trim().parse::<f32>().unwrap_or(0.0)
}

fn parse_f64(value: &str) -> f64 {
    value.trim().parse::<f64>().unwrap_or(0.0)
}

fn round2(value: f32) -> f32 {
    (value * 100.0).round() / 100.0
}

fn round3(value: f32) -> f32 {
    (value * 1000.0).round() / 1000.0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_close(actual: f32, expected: f32) {
        assert!(
            (actual - expected).abs() < 0.001,
            "expected {expected}, got {actual}"
        );
    }

    #[test]
    fn normalize_code_accepts_prefixed_and_plain_codes() {
        assert_eq!(normalize_code("sh600519"), Some("sh600519".to_string()));
        assert_eq!(normalize_code(" 000001 "), Some("sz000001".to_string()));
        assert_eq!(normalize_code("688981"), Some("sh688981".to_string()));
        assert_eq!(normalize_code(""), None);
        assert_eq!(normalize_code("0"), None);
        assert_eq!(normalize_code("abc"), None);
    }

    #[test]
    fn eastmoney_secid_maps_market_prefixes() {
        assert_eq!(eastmoney_secid("sh600519"), "1.600519");
        assert_eq!(eastmoney_secid("sz000001"), "0.000001");
    }

    #[test]
    fn theoretical_limits_follow_board_rules_without_misusing_market_cap_fields() {
        assert_eq!(
            theoretical_price_limits("sh600031", "三一重工", 18.57),
            (20.43, 16.71)
        );
        assert_eq!(
            theoretical_price_limits("sz300001", "特锐德", 10.0),
            (12.0, 8.0)
        );
        assert_eq!(
            theoretical_price_limits("sh600001", "*ST示例", 10.0),
            (0.0, 0.0)
        );
    }

    #[test]
    fn board_page_parser_preserves_total_and_market_prefix() {
        let json = serde_json::json!({
            "data": {
                "total": 101,
                "diff": [
                    {"f12": "600001", "f13": 1, "f14": "A"},
                    {"f12": "000001", "f13": 0, "f14": "B"}
                ]
            }
        });
        let (members, total) = parse_board_page(&json).unwrap();
        assert_eq!(total, 101);
        assert_eq!(members[0].code, "sh600001");
        assert_eq!(members[1].code, "sz000001");
    }

    #[test]
    fn parse_tencent_line_computes_change_fields() {
        let mut parts = vec!["0"; 50];
        parts[1] = "Ping An Bank";
        parts[3] = "10.255";
        parts[4] = "10.001";
        parts[5] = "10.100";
        parts[6] = "1234";
        parts[30] = "20260601150000";
        parts[33] = "10.300";
        parts[34] = "9.950";
        parts[37] = "567.8";
        parts[38] = "1.23";
        parts[45] = "2027.89";
        parts[47] = "11.54";
        parts[48] = "9.44";
        parts[49] = "1.08";

        let data = parse_tencent_line("sz000001", &parts.join("~"));

        assert_eq!(data.code, "sz000001");
        assert_eq!(data.name, "Ping An Bank");
        assert_close(data.price, 10.255);
        assert_close(data.prev_close, 10.001);
        assert_close(data.change, 0.254);
        assert_close(data.change_percent, 2.54);
        assert_close(data.volume, 1234.0);
        assert_close(data.amount, 5_678_000.0);
        assert_close(data.turnover, 1.23);
        assert_close(data.volume_ratio, 1.08);
        assert!((data.float_market_cap - 202_789_000_000.0).abs() < 1.0);
        assert_close(data.upper_limit, 11.54);
        assert_close(data.lower_limit, 9.44);
        assert_eq!(data.date, "2026-06-01");
        assert_eq!(data.time, "15:00:00");
        assert_eq!(data.source, "tencent");
    }
}
