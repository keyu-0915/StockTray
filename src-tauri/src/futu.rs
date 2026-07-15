use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{Duration, Instant};

use prost::Message;
use sha1::{Digest, Sha1};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::time::timeout;

use crate::models::{ExternalDataSourceConfig, StockData};
use crate::quotes::theoretical_price_limits;

const HEADER_LEN: usize = 44;
const INIT_CONNECT_PROTO: u32 = 1001;
const SNAPSHOT_PROTO: u32 = 3203;
const MAX_SNAPSHOT_BATCH: usize = 400;
const IO_TIMEOUT: Duration = Duration::from_secs(5);
const MAX_BODY_LEN: usize = 16 * 1024 * 1024;
static NEXT_SERIAL: AtomicU32 = AtomicU32::new(1);

#[derive(Clone, PartialEq, Message)]
struct InitC2s {
    #[prost(int32, required, tag = "1")]
    client_ver: i32,
    #[prost(string, required, tag = "2")]
    client_id: String,
    #[prost(bool, optional, tag = "3")]
    recv_notify: Option<bool>,
    #[prost(int32, optional, tag = "4")]
    packet_enc_algo: Option<i32>,
    #[prost(int32, optional, tag = "5")]
    push_proto_fmt: Option<i32>,
    #[prost(string, optional, tag = "6")]
    programming_language: Option<String>,
}

#[derive(Clone, PartialEq, Message)]
struct InitRequest {
    #[prost(message, required, tag = "1")]
    c2s: InitC2s,
}

#[derive(Clone, PartialEq, Message)]
struct InitS2c {
    #[prost(int32, required, tag = "1")]
    server_ver: i32,
    #[prost(uint64, required, tag = "2")]
    login_user_id: u64,
    #[prost(uint64, required, tag = "3")]
    conn_id: u64,
    #[prost(string, required, tag = "4")]
    conn_aes_key: String,
    #[prost(int32, required, tag = "5")]
    keep_alive_interval: i32,
}

#[derive(Clone, PartialEq, Message)]
struct InitResponse {
    #[prost(int32, required, tag = "1")]
    ret_type: i32,
    #[prost(string, optional, tag = "2")]
    ret_msg: Option<String>,
    #[prost(message, optional, tag = "4")]
    s2c: Option<InitS2c>,
}

#[derive(Clone, PartialEq, Message)]
struct Security {
    #[prost(int32, required, tag = "1")]
    market: i32,
    #[prost(string, required, tag = "2")]
    code: String,
}

#[derive(Clone, PartialEq, Message)]
struct SnapshotC2s {
    #[prost(message, repeated, tag = "1")]
    security_list: Vec<Security>,
}

#[derive(Clone, PartialEq, Message)]
struct SnapshotRequest {
    #[prost(message, required, tag = "1")]
    c2s: SnapshotC2s,
}

#[derive(Clone, PartialEq, Message)]
struct SnapshotBasicData {
    #[prost(message, required, tag = "1")]
    security: Security,
    #[prost(int32, required, tag = "2")]
    security_type: i32,
    #[prost(bool, required, tag = "3")]
    is_suspend: bool,
    #[prost(string, required, tag = "4")]
    list_time: String,
    #[prost(string, required, tag = "7")]
    update_time: String,
    #[prost(double, required, tag = "8")]
    high_price: f64,
    #[prost(double, required, tag = "9")]
    open_price: f64,
    #[prost(double, required, tag = "10")]
    low_price: f64,
    #[prost(double, required, tag = "11")]
    last_close_price: f64,
    #[prost(double, required, tag = "12")]
    cur_price: f64,
    #[prost(int64, required, tag = "13")]
    volume: i64,
    #[prost(double, required, tag = "14")]
    turnover: f64,
    #[prost(double, required, tag = "15")]
    turnover_rate: f64,
    #[prost(double, optional, tag = "32")]
    volume_ratio: Option<f64>,
    #[prost(string, optional, tag = "41")]
    name: Option<String>,
}

#[derive(Clone, PartialEq, Message)]
struct EquitySnapshotExData {
    #[prost(int64, required, tag = "6")]
    outstanding_shares: i64,
    #[prost(double, required, tag = "7")]
    outstanding_market_val: f64,
}

#[derive(Clone, PartialEq, Message)]
struct Snapshot {
    #[prost(message, required, tag = "1")]
    basic: SnapshotBasicData,
    #[prost(message, optional, tag = "2")]
    equity_ex_data: Option<EquitySnapshotExData>,
}

#[derive(Clone, PartialEq, Message)]
struct SnapshotS2c {
    #[prost(message, repeated, tag = "1")]
    snapshot_list: Vec<Snapshot>,
}

#[derive(Clone, PartialEq, Message)]
struct SnapshotResponse {
    #[prost(int32, required, tag = "1")]
    ret_type: i32,
    #[prost(string, optional, tag = "2")]
    ret_msg: Option<String>,
    #[prost(message, optional, tag = "4")]
    s2c: Option<SnapshotS2c>,
}

struct OpenDClient {
    stream: TcpStream,
    server_ver: i32,
}

impl OpenDClient {
    async fn connect(source: &ExternalDataSourceConfig) -> Result<Self, String> {
        let host = source.host.trim();
        let stream = timeout(IO_TIMEOUT, TcpStream::connect((host, source.port)))
            .await
            .map_err(|_| "连接 OpenD 超时".to_string())?
            .map_err(|error| format!("连接 OpenD 失败：{error}"))?;
        let mut client = Self {
            stream,
            server_ver: 0,
        };
        let request = InitRequest {
            c2s: InitC2s {
                client_ver: 218,
                client_id: "StockTray-Windows".into(),
                recv_notify: Some(false),
                packet_enc_algo: Some(0),
                push_proto_fmt: Some(0),
                programming_language: Some("Rust".into()),
            },
        };
        let body = client.request(INIT_CONNECT_PROTO, request).await?;
        let response = InitResponse::decode(body.as_slice())
            .map_err(|error| format!("OpenD 握手响应解析失败：{error}"))?;
        if response.ret_type != 0 {
            return Err(format!(
                "OpenD 握手失败：{}",
                response.ret_msg.unwrap_or_else(|| "未知错误".into())
            ));
        }
        client.server_ver = response
            .s2c
            .ok_or_else(|| "OpenD 握手响应缺少连接信息".to_string())?
            .server_ver;
        Ok(client)
    }

    async fn request<M: Message>(&mut self, proto_id: u32, message: M) -> Result<Vec<u8>, String> {
        let body = message.encode_to_vec();
        let serial = NEXT_SERIAL.fetch_add(1, Ordering::Relaxed);
        let packet = encode_packet(proto_id, serial, &body);
        timeout(IO_TIMEOUT, self.stream.write_all(&packet))
            .await
            .map_err(|_| "写入 OpenD 请求超时".to_string())?
            .map_err(|error| format!("写入 OpenD 请求失败：{error}"))?;
        let (response_proto, response_serial, body) = read_packet(&mut self.stream).await?;
        if response_proto != proto_id || response_serial != serial {
            return Err(format!(
                "OpenD 响应不匹配：期望 {proto_id}/{serial}，收到 {response_proto}/{response_serial}"
            ));
        }
        Ok(body)
    }

    async fn snapshots(&mut self, codes: &[String]) -> Result<Vec<StockData>, String> {
        let mut quotes = Vec::new();
        for batch in codes.chunks(MAX_SNAPSHOT_BATCH) {
            let securities = batch
                .iter()
                .filter_map(|code| security_from_code(code))
                .collect::<Vec<_>>();
            if securities.is_empty() {
                continue;
            }
            let body = self
                .request(
                    SNAPSHOT_PROTO,
                    SnapshotRequest {
                        c2s: SnapshotC2s {
                            security_list: securities,
                        },
                    },
                )
                .await?;
            let response = SnapshotResponse::decode(body.as_slice())
                .map_err(|error| format!("OpenD 快照响应解析失败：{error}"))?;
            if response.ret_type != 0 {
                return Err(format!(
                    "OpenD 快照失败：{}",
                    response.ret_msg.unwrap_or_else(|| "未知错误".into())
                ));
            }
            let s2c = response
                .s2c
                .ok_or_else(|| "OpenD 快照响应没有数据".to_string())?;
            quotes.extend(s2c.snapshot_list.into_iter().filter_map(snapshot_to_stock));
        }
        Ok(quotes)
    }
}

pub(crate) async fn fetch_quotes(
    source: &ExternalDataSourceConfig,
    codes: &[String],
) -> Result<Vec<StockData>, String> {
    if codes.is_empty() {
        return Ok(Vec::new());
    }
    let mut client = OpenDClient::connect(source).await?;
    client.snapshots(codes).await
}

pub(crate) async fn test_connection(
    source: &ExternalDataSourceConfig,
) -> Result<(u128, String), String> {
    let started = Instant::now();
    let mut client = OpenDClient::connect(source).await?;
    let quotes = client.snapshots(&["sh000001".into()]).await?;
    let quote = quotes
        .first()
        .ok_or_else(|| "OpenD 已连接，但未返回上证指数快照".to_string())?;
    Ok((
        started.elapsed().as_millis(),
        format!(
            "OpenD {} 握手与快照验证通过（{} {:.2}）",
            format_server_version(client.server_ver),
            quote.name,
            quote.price
        ),
    ))
}

fn security_from_code(code: &str) -> Option<Security> {
    let (market, number) = if let Some(number) = code.strip_prefix("sh") {
        (21, number)
    } else {
        (22, code.strip_prefix("sz")?)
    };
    (number.len() == 6 && number.chars().all(|character| character.is_ascii_digit())).then(|| {
        Security {
            market,
            code: number.into(),
        }
    })
}

fn snapshot_to_stock(snapshot: Snapshot) -> Option<StockData> {
    let basic = snapshot.basic;
    let prefix = match basic.security.market {
        21 => "sh",
        22 => "sz",
        _ => return None,
    };
    let code = format!("{prefix}{}", basic.security.code);
    let price = basic.cur_price as f32;
    let prev_close = basic.last_close_price as f32;
    let change = price - prev_close;
    let change_percent = if prev_close > 0.0 {
        change / prev_close * 100.0
    } else {
        0.0
    };
    let name = basic.name.unwrap_or_else(|| basic.security.code.clone());
    let (upper_limit, lower_limit) = theoretical_price_limits(&code, &name, prev_close);
    let (date, time) = split_update_time(&basic.update_time);
    Some(StockData {
        code,
        name,
        price,
        prev_close,
        open: basic.open_price as f32,
        high: basic.high_price as f32,
        low: basic.low_price as f32,
        volume: basic.volume as f32,
        amount: basic.turnover as f32,
        volume_ratio: basic.volume_ratio.unwrap_or_default() as f32,
        change,
        change_percent,
        turnover: basic.turnover_rate as f32,
        float_market_cap: snapshot
            .equity_ex_data
            .map_or(0.0, |data| data.outstanding_market_val),
        upper_limit,
        lower_limit,
        listing_date: basic.list_time,
        source: "futu".into(),
        date,
        time,
        ..Default::default()
    })
}

fn split_update_time(value: &str) -> (String, String) {
    let mut parts = value.split_whitespace();
    (
        parts.next().unwrap_or_default().into(),
        parts.next().unwrap_or_default().into(),
    )
}

fn format_server_version(version: i32) -> String {
    if version >= 100 {
        format!("v{}.{}", version / 100, version % 100)
    } else {
        format!("v{version}")
    }
}

fn encode_packet(proto_id: u32, serial: u32, body: &[u8]) -> Vec<u8> {
    let digest = Sha1::digest(body);
    let mut packet = Vec::with_capacity(HEADER_LEN + body.len());
    packet.extend_from_slice(b"FT");
    packet.extend_from_slice(&proto_id.to_le_bytes());
    packet.push(0);
    packet.push(0);
    packet.extend_from_slice(&serial.to_le_bytes());
    packet.extend_from_slice(&(body.len() as u32).to_le_bytes());
    packet.extend_from_slice(&digest);
    packet.extend_from_slice(&[0; 8]);
    packet.extend_from_slice(body);
    packet
}

async fn read_packet(stream: &mut TcpStream) -> Result<(u32, u32, Vec<u8>), String> {
    let mut header = [0_u8; HEADER_LEN];
    timeout(IO_TIMEOUT, stream.read_exact(&mut header))
        .await
        .map_err(|_| "读取 OpenD 响应超时".to_string())?
        .map_err(|error| format!("读取 OpenD 响应头失败：{error}"))?;
    if &header[0..2] != b"FT" {
        return Err("OpenD 响应包头无效".into());
    }
    let proto_id = u32::from_le_bytes(header[2..6].try_into().expect("fixed header"));
    let serial = u32::from_le_bytes(header[8..12].try_into().expect("fixed header"));
    let body_len = u32::from_le_bytes(header[12..16].try_into().expect("fixed header")) as usize;
    if body_len > MAX_BODY_LEN {
        return Err(format!("OpenD 响应体异常：{body_len} 字节"));
    }
    let mut body = vec![0_u8; body_len];
    timeout(IO_TIMEOUT, stream.read_exact(&mut body))
        .await
        .map_err(|_| "读取 OpenD 响应内容超时".to_string())?
        .map_err(|error| format!("读取 OpenD 响应内容失败：{error}"))?;
    if Sha1::digest(&body).as_slice() != &header[16..36] {
        return Err("OpenD 响应 SHA-1 校验失败".into());
    }
    Ok((proto_id, serial, body))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn packet_header_is_little_endian_and_sha_checked() {
        let body = [1_u8, 2, 3];
        let packet = encode_packet(3203, 42, &body);
        assert_eq!(&packet[0..2], b"FT");
        assert_eq!(&packet[2..6], &3203_u32.to_le_bytes());
        assert_eq!(&packet[8..12], &42_u32.to_le_bytes());
        assert_eq!(&packet[12..16], &3_u32.to_le_bytes());
        assert_eq!(&packet[16..36], Sha1::digest(body).as_slice());
        assert_eq!(&packet[HEADER_LEN..], body);
    }

    #[test]
    fn a_share_codes_map_to_official_markets() {
        assert_eq!(security_from_code("sh600519").unwrap().market, 21);
        assert_eq!(security_from_code("sz300750").unwrap().market, 22);
        assert!(security_from_code("em:000001").is_none());
    }

    #[test]
    fn update_time_is_split_without_timezone_guessing() {
        assert_eq!(
            split_update_time("2026-07-15 14:35:00"),
            ("2026-07-15".into(), "14:35:00".into())
        );
    }

    #[test]
    #[ignore = "requires FUTU_OPEND_ENDPOINT and a reachable OpenD"]
    fn live_opend_returns_complete_a_share_snapshots() {
        let endpoint = std::env::var("FUTU_OPEND_ENDPOINT")
            .expect("set FUTU_OPEND_ENDPOINT, for example 192.168.10.33:32179");
        let (host, port) = endpoint
            .rsplit_once(':')
            .expect("FUTU_OPEND_ENDPOINT must be host:port");
        let source = ExternalDataSourceConfig {
            id: "live-test".into(),
            provider: "futu_opend".into(),
            name: "Futu OpenD".into(),
            host: host.into(),
            port: port.parse().expect("port must be numeric"),
            enabled: true,
        };
        let (_, verification) =
            tauri::async_runtime::block_on(test_connection(&source)).expect("connection test");
        println!("{verification}");
        let quotes = tauri::async_runtime::block_on(fetch_quotes(
            &source,
            &["sh600519".into(), "sz300750".into(), "sh000001".into()],
        ))
        .expect("OpenD snapshot request should succeed");
        assert_eq!(quotes.len(), 3);
        for quote in quotes {
            assert!(quote.price > 0.0, "{} should have a price", quote.code);
            assert!(
                quote.prev_close > 0.0,
                "{} should have prev close",
                quote.code
            );
            assert!(!quote.name.is_empty(), "{} should have a name", quote.code);
            assert!(!quote.date.is_empty(), "{} should have a date", quote.code);
            assert_eq!(quote.source, "futu");
        }
    }
}
