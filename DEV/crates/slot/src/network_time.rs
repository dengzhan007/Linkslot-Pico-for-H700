//! Lightweight SNTP time synchronisation.
//!
//! The handheld has Wi-Fi but no reliable battery-backed clock, and the
//! on-screen clock is also used for timestamps and save ordering.  Once Wi-Fi
//! is connected we ask a public SNTP server for UTC in a background thread;
//! the UI thread never blocks on DNS or UDP.

use std::net::{ToSocketAddrs, UdpSocket};
use std::sync::mpsc::{self, Receiver};
use std::time::Duration;

const NTP_PORT: u16 = 123;
const NTP_UNIX_OFFSET: i64 = 2_208_988_800;
const TIMEOUT: Duration = Duration::from_secs(2);
const HOSTS: &[&str] = &["pool.ntp.org", "time.cloudflare.com", "ntp.aliyun.com"];
const IP_TIME_URLS: [(&str, fn(&str) -> Result<(i16, String), String>); 2] = [
    ("http://ip-api.com/json/?fields=status,timezone,offset", parse_timezone),
    ("http://ipwho.is/", parse_timezone_ipwho),
];
const IP_TIME_TIMEOUT: Duration = Duration::from_secs(5);

/// One network clock result.  `offset_min` is present only when the IP
/// geolocation lookup returned a usable zone; the SNTP result alone carries no
/// timezone information.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClockSync {
    pub utc: i64,
    pub offset_min: Option<i16>,
    pub timezone: Option<String>,
}

/// Spawn one best-effort SNTP lookup.  The receiver yields Unix UTC seconds.
pub fn spawn() -> Receiver<Result<i64, String>> {
    let (tx, rx) = mpsc::channel();
    let _ = std::thread::Builder::new()
        .name("slot-ntp".into())
        .spawn(move || {
            let _ = tx.send(sync());
        });
    rx
}

/// Spawn SNTP plus an IP-based timezone lookup.  The timezone lookup is
/// best-effort: a blocked geolocation service must not prevent the UTC clock
/// from being corrected.
pub fn spawn_clock() -> Receiver<Result<ClockSync, String>> {
    let (tx, rx) = mpsc::channel();
    let _ = std::thread::Builder::new()
        .name("slot-clock".into())
        .spawn(move || {
            let result = sync().map(|utc| {
                let (offset_min, timezone) = match lookup_timezone() {
                    Ok((offset, zone)) => (Some(offset), Some(zone)),
                    Err(_) => (None, None),
                };
                ClockSync {
                    utc,
                    offset_min,
                    timezone,
                }
            });
            let _ = tx.send(result);
        });
    rx
}

/// Resolve a timezone offset from the public IP.  Uses the free ip-api
/// endpoint (HTTP, no account).  A failed lookup is not fatal to time sync.
pub fn lookup_timezone() -> Result<(i16, String), String> {
    let config = ureq::Agent::config_builder()
        .timeout_global(Some(IP_TIME_TIMEOUT))
        .max_redirects(1)
        .http_status_as_error(false)
        .build();
    let agent: ureq::Agent = config.into();
    let mut errors = Vec::new();
    for (url, parse) in IP_TIME_URLS {
        let result = (|| -> Result<(i16, String), String> {
            let mut response = agent
                .get(url)
                .call()
                .map_err(|error| format!("请求 {url} 失败：{error}"))?;
            if !(200..300).contains(&response.status().as_u16()) {
                return Err(format!("IP 定位 HTTP {}", response.status().as_u16()));
            }
            let body = response
                .body_mut()
                .with_config()
                .limit(16 * 1024)
                .read_to_string()
                .map_err(|error| format!("IP 定位响应读取失败：{error}"))?;
            parse(&body)
        })();
        match result {
            Ok(found) => return Ok(found),
            Err(error) => errors.push(error),
        }
    }
    Err(errors.join("; "))
}

/// Parse the ipwho.is JSON, whose timezone block nests the offset in seconds.
pub fn parse_timezone_ipwho(body: &str) -> Result<(i16, String), String> {
    let value: serde_json::Value =
        serde_json::from_str(body).map_err(|error| format!("IP 定位 JSON 无效：{error}"))?;
    if value.get("success").and_then(|v| v.as_bool()) != Some(true) {
        return Err("IP 定位没有返回 success".into());
    }
    let zone = value
        .get("timezone")
        .ok_or_else(|| "IP 定位缺少 timezone".to_string())?;
    let offset = zone
        .get("offset")
        .and_then(|v| v.as_i64())
        .ok_or_else(|| "IP 定位缺少 offset".to_string())?;
    let offset = i16::try_from(offset / 60).map_err(|_| "IP 定位时区超出范围".to_string())?;
    if !(-720..=840).contains(&offset) {
        return Err("IP 定位时区超出范围".into());
    }
    let timezone = zone
        .get("id")
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string();
    Ok((offset, timezone))
}

/// Parse the ip-api JSON.  Split out for tests so no network is needed.
pub fn parse_timezone(body: &str) -> Result<(i16, String), String> {
    let value: serde_json::Value =
        serde_json::from_str(body).map_err(|error| format!("IP 定位 JSON 无效：{error}"))?;
    if value.get("status").and_then(|v| v.as_str()) != Some("success") {
        return Err("IP 定位没有返回 success".into());
    }
    let offset = value
        .get("offset")
        .and_then(|v| v.as_i64())
        .ok_or_else(|| "IP 定位缺少 offset".to_string())?;
    let offset = i16::try_from(offset / 60).map_err(|_| "IP 定位时区超出范围".to_string())?;
    if !(-720..=840).contains(&offset) {
        return Err("IP 定位时区超出范围".into());
    }
    let timezone = value
        .get("timezone")
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string();
    Ok((offset, timezone))
}

/// Query the first reachable SNTP host.
pub fn sync() -> Result<i64, String> {
    let mut errors = Vec::new();
    for host in HOSTS {
        match query(host) {
            Ok(seconds) => return Ok(seconds),
            Err(error) => errors.push(format!("{host}: {error}")),
        }
    }
    Err(errors.join("; "))
}

fn query(host: &str) -> Result<i64, String> {
    let address = (host, NTP_PORT)
        .to_socket_addrs()
        .map_err(|error| format!("DNS: {error}"))?
        .find(|address| address.is_ipv4())
        .ok_or_else(|| "没有 IPv4 地址".to_string())?;
    let socket = UdpSocket::bind("0.0.0.0:0").map_err(|error| error.to_string())?;
    socket
        .set_read_timeout(Some(TIMEOUT))
        .map_err(|error| error.to_string())?;
    socket
        .set_write_timeout(Some(TIMEOUT))
        .map_err(|error| error.to_string())?;
    socket
        .connect(address)
        .map_err(|error| error.to_string())?;

    // LI=0, VN=3, mode=3 (client).
    let mut request = [0u8; 48];
    request[0] = 0x1b;
    socket.send(&request).map_err(|error| error.to_string())?;

    let mut reply = [0u8; 48];
    let length = socket.recv(&mut reply).map_err(|error| error.to_string())?;
    if length < 48 {
        return Err("NTP 响应过短".into());
    }
    parse_reply(&reply).ok_or_else(|| "NTP 响应无效".into())
}

fn parse_reply(reply: &[u8; 48]) -> Option<i64> {
    if reply[0] & 0x07 != 4 || reply[1] == 0 || reply[1] >= 16 {
        return None;
    }
    let seconds = u32::from_be_bytes(reply[40..44].try_into().ok()?) as i64;
    let unix = seconds - NTP_UNIX_OFFSET;
    // Guard against an unset server timestamp or a spoofed/foreign packet.
    (1_577_836_800..=4_102_444_800).contains(&unix).then_some(unix)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_valid_sntp_reply_becomes_unix_seconds() {
        let mut reply = [0u8; 48];
        reply[0] = 0x24; // LI=0, VN=4, mode=4 (server)
        reply[1] = 2;
        reply[40..44].copy_from_slice(&(NTP_UNIX_OFFSET as u32 + 1_700_000_000).to_be_bytes());
        assert_eq!(parse_reply(&reply), Some(1_700_000_000));
    }

    #[test]
    fn a_client_packet_is_not_accepted_as_a_reply() {
        let reply = [0x1b; 48];
        assert_eq!(parse_reply(&reply), None);
    }

    #[test]
    fn ip_timezone_offsets_are_converted_from_seconds_to_minutes() {
        let body = r#"{"status":"success","offset":28800,"timezone":"Asia/Shanghai"}"#;
        assert_eq!(parse_timezone(body), Ok((480, "Asia/Shanghai".into())));
    }

    #[test]
    fn a_failed_ip_lookup_is_an_error() {
        assert!(parse_timezone(r#"{"status":"fail"}"#).is_err());
        assert!(parse_timezone_ipwho(r#"{"success":false}"#).is_err());
    }

    #[test]
    fn ipwho_timezone_offsets_are_read_from_the_nested_block() {
        let body = r#"{"success":true,"timezone":{"id":"Asia/Shanghai","offset":28800}}"#;
        assert_eq!(parse_timezone_ipwho(body), Ok((480, "Asia/Shanghai".into())));
    }
}
