//! Same-Wi-Fi link discovery.
//!
//! In `Hotspot` mode the host creates the `slotlink` AP and the peer joins it,
//! so the host address is always `10.42.0.1`.  In `Wifi` mode both handhelds
//! stay on the user's router; the host listens on all interfaces and answers a
//! small UDP broadcast so the joiner can find its LAN address.

use std::net::{Ipv4Addr, SocketAddr, UdpSocket};
use std::time::{Duration, Instant};

use crate::link_net::Cancel;

const MAGIC: &[u8] = b"SLOTLINK?";
const REPLY: &[u8] = b"SLOTLINK!";
const POLL_MS: u64 = 120;
const BROADCAST_EVERY: Duration = Duration::from_millis(500);
const DISCOVERY_BOUND: Duration = Duration::from_secs(12);

/// The host address the peer should use. The source address of the UDP reply
/// is used, not anything in the packet, so a multi-homed host still answers
/// with the interface the joiner can reach.
pub fn discover(port: u16, cancel: &Cancel) -> Option<Ipv4Addr> {
    let socket = UdpSocket::bind("0.0.0.0:0").ok()?;
    socket.set_broadcast(true).ok()?;
    socket
        .set_read_timeout(Some(Duration::from_millis(POLL_MS)))
        .ok()?;
    let target = SocketAddr::from((Ipv4Addr::BROADCAST, port));
    let started = Instant::now();
    let mut next_send = Instant::now();
    let mut buf = [0u8; 64];
    loop {
        if cancel.is_cancelled() || started.elapsed() >= DISCOVERY_BOUND {
            return None;
        }
        if Instant::now() >= next_send {
            let _ = socket.send_to(MAGIC, target);
            next_send = Instant::now() + BROADCAST_EVERY;
        }
        match socket.recv_from(&mut buf) {
            Ok((len, from)) if len == REPLY.len() && &buf[..len] == REPLY => {
                if let std::net::IpAddr::V4(ip) = from.ip() {
                    return Some(ip);
                }
            }
            _ => {}
        }
    }
}

/// Spawn a tiny UDP responder. It stops when `cancel` is set; the TCP host
/// listener owns the same cancel flag, so the advertiser dies with the link.
pub fn advertise(port: u16, cancel: Cancel) {
    std::thread::spawn(move || {
        let Ok(socket) = UdpSocket::bind(("0.0.0.0", port)) else {
            crate::link_log::warn(format!("link wifi: discovery port {port} unavailable"));
            return;
        };
        let _ = socket.set_read_timeout(Some(Duration::from_millis(POLL_MS)));
        let mut buf = [0u8; 64];
        while !cancel.is_cancelled() {
            let Ok((len, from)) = socket.recv_from(&mut buf) else {
                continue;
            };
            if len == MAGIC.len() && &buf[..len] == MAGIC {
                let _ = socket.send_to(REPLY, from);
            }
        }
    });
}
