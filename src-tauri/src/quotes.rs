use std::time::Duration;

use encoding_rs::GBK;
use regex::Regex;
use reqwest::Client;

use crate::models::StockData;

pub(crate) async fn fetch_quotes(codes: &[String]) -> Result<Vec<StockData>, String> {
    if codes.is_empty() {
        return Ok(Vec::new());
    }
    let client = http_client()?;
    match fetch_eastmoney(&client, codes).await {
        Ok(data) => Ok(data),
        Err(_) => fetch_sina(&client, codes).await,
    }
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
        "https://push2.eastmoney.com/api/qt/ulist.np/get?fltt=2&fields=f2,f3,f4,f5,f6,f8,f10,f12,f14,f15,f16,f17,f18&secids={}&_={}",
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
                change = round2(price - prev_close);
            }
            if change_percent == 0.0 && prev_close > 0.0 && price > 0.0 {
                change_percent = round2((price - prev_close) / prev_close * 100.0);
            }
            result.push(StockData {
                code: full,
                name: item["f14"].as_str().unwrap_or("").to_string(),
                price,
                prev_close,
                open: json_f32(item, "f17"),
                high: json_f32(item, "f15"),
                low: json_f32(item, "f16"),
                volume: json_f32(item, "f5"),
                amount: json_f32(item, "f6") / 10000.0,
                volume_ratio: json_f32(item, "f10"),
                change,
                change_percent,
                turnover: json_f32(item, "f8"),
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

async fn fetch_sina(client: &Client, codes: &[String]) -> Result<Vec<StockData>, String> {
    let url = format!("http://hq.sinajs.cn/list={}", codes.join(","));
    let bytes = client
        .get(url)
        .header("Referer", "https://finance.sina.com.cn/")
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
        let pattern = format!(r#"var hq_str_{}="([^"]*)""#, regex::escape(code));
        if let Some(caps) = Regex::new(&pattern)
            .map_err(|e| e.to_string())?
            .captures(&text)
        {
            result.push(parse_sina_line(code, &caps[1]));
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

fn parse_sina_line(code: &str, line: &str) -> StockData {
    let parts = line.split(',').collect::<Vec<_>>();
    if parts.len() < 32 {
        return StockData {
            code: code.into(),
            error: "incomplete_data".into(),
            ..Default::default()
        };
    }
    let price = parse_f32(parts[3]);
    let prev_close = parse_f32(parts[2]);
    let change = if prev_close > 0.0 && price > 0.0 {
        round2(price - prev_close)
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
        name: parts[0].into(),
        price,
        prev_close,
        open: parse_f32(parts[1]),
        high: parse_f32(parts[4]),
        low: parse_f32(parts[5]),
        volume: parse_f32(parts[8]) / 100.0,
        amount: parse_f32(parts[9]),
        change,
        change_percent,
        date: parts[30].into(),
        time: parts[31].into(),
        ..Default::default()
    }
}

pub(crate) fn normalize_code(input: &str) -> Option<String> {
    let mut code = input.trim().to_lowercase();
    if code.starts_with("sh") || code.starts_with("sz") {
        return (code.len() == 8 && code[2..].chars().all(|c| c.is_ascii_digit())).then_some(code);
    }
    while code.len() < 6 {
        code = format!("0{code}");
    }
    if code.len() != 6 || !code.chars().all(|c| c.is_ascii_digit()) {
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

fn parse_f32(value: &str) -> f32 {
    value.trim().parse::<f32>().unwrap_or(0.0)
}

fn round2(value: f32) -> f32 {
    (value * 100.0).round() / 100.0
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
        assert_eq!(normalize_code("abc"), None);
    }

    #[test]
    fn eastmoney_secid_maps_market_prefixes() {
        assert_eq!(eastmoney_secid("sh600519"), "1.600519");
        assert_eq!(eastmoney_secid("sz000001"), "0.000001");
    }

    #[test]
    fn parse_sina_line_computes_change_fields() {
        let mut parts = vec!["0"; 32];
        parts[0] = "Ping An Bank";
        parts[1] = "10.10";
        parts[2] = "10.00";
        parts[3] = "10.25";
        parts[4] = "10.30";
        parts[5] = "9.95";
        parts[8] = "123400";
        parts[9] = "5678000";
        parts[30] = "2026-06-01";
        parts[31] = "15:00:00";

        let data = parse_sina_line("sz000001", &parts.join(","));

        assert_eq!(data.code, "sz000001");
        assert_eq!(data.name, "Ping An Bank");
        assert_close(data.price, 10.25);
        assert_close(data.prev_close, 10.00);
        assert_close(data.change, 0.25);
        assert_close(data.change_percent, 2.50);
        assert_close(data.volume, 1234.0);
        assert_close(data.amount, 5_678_000.0);
        assert_eq!(data.date, "2026-06-01");
        assert_eq!(data.time, "15:00:00");
    }
}
