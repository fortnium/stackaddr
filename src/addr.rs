use bytes::Bytes;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use uuid::Uuid;
use std::{fmt, net::{IpAddr, Ipv4Addr, Ipv6Addr}, str::FromStr};
use crate::{error::StackAddrError, segment::{identity::Identity, protocol::Protocol, Segment}};

/// A stack address that contains a stack of protocols.
/// The stack address can be used to represent a network address with multiple protocols.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct StackAddr {
    segments: Vec<Segment>,
}

impl StackAddr {
    pub fn new(segments: Vec<Segment>) -> Self {
        StackAddr { segments }
    }

    pub fn from_parts(segments: &[Segment]) -> Self {
        StackAddr {
            segments: segments.to_vec(),
        }
    }

    pub fn empty() -> Self {
        StackAddr { segments: Vec::new() }
    }

    pub fn with(mut self, segment: Segment) -> Self {
        self.segments.push(segment);
        self
    }

    pub fn with_protocol(mut self, protocol: Protocol) -> Self {
        self.segments.push(Segment::Protocol(protocol));
        self
    }

    pub fn with_identity(mut self, identity: Identity) -> Self {
        self.segments.push(Segment::Identity(identity));
        self
    }

    pub fn with_path(mut self, path: &str) -> Self {
        self.segments.push(Segment::Path(path.to_string()));
        self
    }

    pub fn with_meta(mut self, key: &str, value: &str) -> Self {
        self.segments.push(Segment::Metadata(key.to_string(), value.to_string()));
        self
    }

    pub fn with_mac(mut self, addr: &str) -> Self {
        self.segments.push(Segment::Protocol(Protocol::Mac(addr.parse().unwrap())));
        self
    }

    pub fn with_ipv4(mut self, addr: Ipv4Addr) -> Self {
        self.segments.push(Segment::Protocol(Protocol::Ip4(addr)));
        self
    }

    pub fn with_ipv6(mut self, addr: Ipv6Addr) -> Self {
        self.segments.push(Segment::Protocol(Protocol::Ip6(addr)));
        self
    }

    pub fn with_ip(mut self, addr: IpAddr) -> Self {
        match addr {
            IpAddr::V4(v4) => self.segments.push(Segment::Protocol(Protocol::Ip4(v4))),
            IpAddr::V6(v6) => self.segments.push(Segment::Protocol(Protocol::Ip6(v6))),
        }
        self
    }

    pub fn with_dns_name(mut self, name: &str) -> Self {
        self.segments.push(Segment::Protocol(Protocol::Dns(name.to_string())));
        self
    }

    pub fn segments(&self) -> &[Segment] {
        &self.segments
    }

    pub fn push(&mut self, segment: Segment) {
        self.segments.push(segment);
    }

    pub fn pop(&mut self) -> Option<Segment> {
        self.segments.pop()
    }

    pub fn contains(&self, target: &Segment) -> bool {
        self.segments.contains(target)
    }

    pub fn replace(&mut self, old: &Segment, new: Segment) -> bool {
        if let Some(pos) = self.segments.iter().position(|s| s == old) {
            self.segments[pos] = new;
            true
        } else {
            false
        }
    }

    pub fn replace_all(&mut self, old: &Segment, new: Segment) -> usize {
        let mut count = 0;
        for s in &mut self.segments {
            if s == old {
                *s = new.clone();
                count += 1;
            }
        }
        count
    }

    pub fn remove(&mut self, target: &Segment) -> bool {
        if let Some(pos) = self.segments.iter().position(|s| s == target) {
            self.segments.remove(pos);
            true
        } else {
            false
        }
    }

    pub fn remove_all(&mut self, target: &Segment) -> usize {
        let before = self.segments.len();
        self.segments.retain(|s| s != target);
        before - self.segments.len()
    }

    pub fn ip(&self) -> Option<IpAddr> {
        for seg in &self.segments {
            if let Segment::Protocol(p) = seg {
                match p {
                    Protocol::Ip4(addr) => return Some(IpAddr::V4(*addr)),
                    Protocol::Ip6(addr) => return Some(IpAddr::V6(*addr)),
                    _ => {}
                }
            }
        }
        None
    }

    pub fn port(&self) -> Option<u16> {
        for seg in self.segments.iter().rev() {
            if let Segment::Protocol(p) = seg {
                match p {
                    Protocol::Tcp(p) => return Some(*p),
                    Protocol::Udp(p) => return Some(*p),
                    Protocol::Quic => return Some(443),
                    Protocol::Ws(p) => return Some(*p),
                    Protocol::Wss(p) => return Some(*p),
                    Protocol::WebTransport(p) => return Some(*p),
                    _ => {}
                }
            }
        }
        None
    }

    pub fn name(&self) -> Option<&str> {
        for seg in &self.segments {
            if let Segment::Protocol(p) = seg {
                match p {
                    Protocol::Dns(name) | Protocol::Dns4(name) | Protocol::Dns6(name) => {
                        return Some(name)
                    }
                    _ => {}
                }
            }
        }
        None
    }

    pub fn resolved(&self) -> bool {
        self.segments.iter().any(|seg| matches!(seg,
            Segment::Protocol(Protocol::Ip4(_) | Protocol::Ip6(_))
        ))
    }

    pub fn is_empty(&self) -> bool {
        self.segments.is_empty()
    }
}

impl fmt::Display for StackAddr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for seg in &self.segments {
            write!(f, "{}", seg)?;
        }
        Ok(())
    }
}

impl FromStr for StackAddr {
    type Err = StackAddrError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut segments = Vec::new();
        let mut parts = s.split('/').filter(|p| !p.is_empty());

        while let Some(part) = parts.next() {
            let seg = match part {
                "ip4" => Segment::Protocol(Protocol::Ip4(parts.next().ok_or(StackAddrError::MissingPart("ip4 address"))?.parse()?)),
                "ip6" => Segment::Protocol(Protocol::Ip6(parts.next().ok_or(StackAddrError::MissingPart("ip6 address"))?.parse()?)),
                "dns" => Segment::Protocol(Protocol::Dns(parts.next().ok_or(StackAddrError::MissingPart("dns"))?.to_string())),
                "dns4" => Segment::Protocol(Protocol::Dns4(parts.next().ok_or(StackAddrError::MissingPart("dns4"))?.to_string())),
                "dns6" => Segment::Protocol(Protocol::Dns6(parts.next().ok_or(StackAddrError::MissingPart("dns6"))?.to_string())),
                "mac" => Segment::Protocol(Protocol::Mac(parts.next().ok_or(StackAddrError::MissingPart("mac address"))?.parse().map_err(|_e| StackAddrError::InvalidEncoding("mac"))?)),
                "tcp" => Segment::Protocol(Protocol::Tcp(parts.next().ok_or(StackAddrError::MissingPart("tcp port"))?.parse()?)),
                "udp" => Segment::Protocol(Protocol::Udp(parts.next().ok_or(StackAddrError::MissingPart("udp port"))?.parse()?)),
                "tls" => Segment::Protocol(Protocol::Tls),
                "quic" => Segment::Protocol(Protocol::Quic),
                "http" => Segment::Protocol(Protocol::Http),
                "https" => Segment::Protocol(Protocol::Https),
                "ws" => Segment::Protocol(Protocol::Ws(parts.next().ok_or(StackAddrError::MissingPart("ws port"))?.parse()?)),
                "wss" => Segment::Protocol(Protocol::Wss(parts.next().ok_or(StackAddrError::MissingPart("wss port"))?.parse()?)),
                "wtr" => Segment::Protocol(Protocol::WebTransport(parts.next().ok_or(StackAddrError::MissingPart("wtr port"))?.parse()?)),
                "webrtc" => Segment::Protocol(Protocol::WebRTC),
                "onion" => Segment::Protocol(Protocol::Onion(parts.next().ok_or(StackAddrError::MissingPart("onion address"))?.to_string())),
                "custom" => Segment::Protocol(Protocol::Custom(parts.next().ok_or(StackAddrError::MissingPart("custom name"))?.to_string())),
                "node" => {
                    let encoded = parts.next().ok_or(StackAddrError::MissingPart("node id"))?;
                    let decoded = base32::decode(base32::Alphabet::Rfc4648 { padding: false }, encoded).ok_or(StackAddrError::InvalidEncoding("base32 node id"))?;
                    Segment::Identity(Identity::NodeId(Bytes::from(decoded)))
                }
                "peer" => {
                    let encoded = parts.next().ok_or(StackAddrError::MissingPart("peer id"))?;
                    let decoded = base32::decode(base32::Alphabet::Rfc4648 { padding: false }, encoded).ok_or(StackAddrError::InvalidEncoding("base32 peer id"))?;
                    Segment::Identity(Identity::PeerId(Bytes::from(decoded)))
                }
                "uuid" => {
                    let val = parts.next().ok_or(StackAddrError::MissingPart("uuid value"))?;
                    let uuid = Uuid::parse_str(val).map_err(|_| StackAddrError::InvalidEncoding("uuid"))?;
                    Segment::Identity(Identity::Uuid(uuid))
                }
                "identity" => {
                    let kind = parts.next().ok_or(StackAddrError::MissingPart("identity kind"))?;
                    let encoded = parts.next().ok_or(StackAddrError::MissingPart("identity value"))?;
                    let decoded = base32::decode(base32::Alphabet::Rfc4648 { padding: false }, encoded).ok_or(StackAddrError::InvalidEncoding("base32 identity"))?;
                    Segment::Identity(Identity::Custom {
                        kind: kind.to_string(),
                        id: Bytes::from(decoded),
                    })
                }
                "meta" => {
                    let k = parts.next().ok_or(StackAddrError::MissingPart("metadata key"))?;
                    let v = parts.next().ok_or(StackAddrError::MissingPart("metadata value"))?;
                    Segment::Metadata(k.to_string(), v.to_string())
                }
                s => Segment::Path(s.to_string()),
            };
            segments.push(seg);
        }

        Ok(StackAddr { segments })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::segment::{protocol::Protocol, identity::Identity, Segment};
    use bytes::Bytes;
    use std::net::{IpAddr, Ipv6Addr};

    fn random_bytes32() -> Bytes {
        use rand::RngCore;
        let mut buf = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut buf);
        Bytes::copy_from_slice(&buf)
    }

    #[test]
    fn test_protocol_display() {
        let s = "/ip4/127.0.0.1/tcp/443/tls/http";
        let addr: StackAddr = s.parse().unwrap();
        assert_eq!(addr.to_string(), s);
    }

    #[test]
    fn test_builder_and_display() {
        let addr = StackAddr::empty()
            .with_protocol(Protocol::Ip4("192.168.10.10".parse().unwrap()))
            .with_protocol(Protocol::Udp(4433))
            .with_protocol(Protocol::Quic);

        assert_eq!(addr.to_string(), "/ip4/192.168.10.10/udp/4433/quic");
    }

    #[test]
    fn test_parse_from_str() {
        let addr: StackAddr = "/ip6/::1/tcp/8080/http".parse().unwrap();
        assert_eq!(addr.ip().unwrap(), IpAddr::V6(Ipv6Addr::LOCALHOST));
        assert_eq!(addr.port(), Some(8080));
    }

    #[test]
    fn test_identity_nodeid() {
        let id = random_bytes32();
        let addr = StackAddr::empty().with_identity(Identity::NodeId(id.clone()));
        assert_eq!(addr.segments().last(), Some(&Segment::Identity(Identity::NodeId(id))));
    }

    #[test]
    fn test_identity_uuid() {
        let s = "/uuid/550e8400-e29b-41d4-a716-446655440000";
        let addr: StackAddr = s.parse().unwrap();
        assert!(matches!(addr.segments().last(), Some(Segment::Identity(Identity::Uuid(_)))));
    }

    #[test]
    fn test_identity_custom() {
        let id = random_bytes32();
        let encoded = base32::encode(base32::Alphabet::Rfc4648 { padding: false }, &id);
        let s = format!("/identity/myproto/{}", encoded);
        let addr: StackAddr = s.parse().unwrap();
        assert!(matches!(addr.segments().last(), Some(Segment::Identity(Identity::Custom { .. }))));
    }

    #[test]
    fn test_metadata_segment() {
        let addr: StackAddr = "/meta/env/production".parse().unwrap();
        assert_eq!(
            addr.segments(),
            &[Segment::Metadata("env".into(), "production".into())]
        );
    }

    #[test]
    fn test_path_segment() {
        let addr: StackAddr = "/downloads/images".parse().unwrap();
        assert_eq!(
            addr.segments(),
            &[Segment::Path("downloads".into()), Segment::Path("images".into())]
        );
    }

    #[test]
    fn test_l2_to_l4() {
        let s = "/mac/aa:bb:cc:dd:ee:ff/ip4/192.168.1.1/tcp/8080";
        let addr: StackAddr = s.parse().expect("parse failed");

        let expected = StackAddr::from_parts(&[
            Segment::Protocol(Protocol::Mac("aa:bb:cc:dd:ee:ff".parse().unwrap())),
            Segment::Protocol(Protocol::Ip4("192.168.1.1".parse().unwrap())),
            Segment::Protocol(Protocol::Tcp(8080)),
        ]);

        assert_eq!(addr, expected);
    }

    #[test]
    fn test_error_display() {
        let err = StackAddrError::MissingPart("foo");
        assert_eq!(err.to_string(), "Missing foo");
    }

    #[cfg(feature = "serde")]
    #[test]
    fn test_serde() {
        let id = random_bytes32();
        let addr = StackAddr::from_parts(&[
            Segment::Protocol(Protocol::Ip4("127.0.0.1".parse().unwrap())),
            Segment::Protocol(Protocol::Tcp(443)),
            Segment::Identity(Identity::NodeId(id.clone())),
            Segment::Metadata("env".into(), "prod".into()),
        ]);

        let json = serde_json::to_string(&addr).unwrap();
        let deserialized: StackAddr = serde_json::from_str(&json).unwrap();
        assert_eq!(addr, deserialized);
    }
}
