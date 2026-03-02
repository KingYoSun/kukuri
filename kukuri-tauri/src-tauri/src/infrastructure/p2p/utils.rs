use crate::shared::error::AppError;
use iroh::{EndpointAddr, EndpointId, RelayUrl};
use std::net::{IpAddr, Ipv4Addr, SocketAddr, ToSocketAddrs};
use std::str::FromStr;

#[derive(Debug, Clone)]
pub struct ParsedPeer {
    pub node_id: EndpointId,
    pub node_addr: Option<EndpointAddr>,
}

fn parse_endpoint_id(value: &str) -> Result<EndpointId, AppError> {
    EndpointId::from_str(value.trim())
        .map_err(|e| AppError::P2PError(format!("Failed to parse node ID: {e}")))
}

fn build_endpoint_addr(
    node_id: EndpointId,
    socket_addrs: Vec<SocketAddr>,
    relay_urls: Vec<RelayUrl>,
) -> Option<EndpointAddr> {
    if socket_addrs.is_empty() && relay_urls.is_empty() {
        return None;
    }

    let mut endpoint_addr = EndpointAddr::new(node_id);
    for relay_url in relay_urls {
        endpoint_addr = endpoint_addr.with_relay_url(relay_url);
    }
    for socket_addr in socket_addrs {
        endpoint_addr = endpoint_addr.with_ip_addr(socket_addr);
    }

    Some(endpoint_addr)
}

fn parse_extended_peer_hint(value: &str) -> Result<ParsedPeer, AppError> {
    let mut segments = value
        .split('|')
        .map(|segment| segment.trim())
        .filter(|segment| !segment.is_empty());

    let first_segment = segments
        .next()
        .ok_or_else(|| AppError::P2PError(format!("Invalid peer hint format: {value}")))?;

    let (node_id, initial_addr) =
        if let Some((node_part, addr_part)) = first_segment.split_once('@') {
            (parse_endpoint_id(node_part)?, Some(addr_part.trim()))
        } else if first_segment.contains('=') {
            return Err(AppError::P2PError(format!(
                "Peer hint is missing node ID before attributes: {value}"
            )));
        } else {
            (parse_endpoint_id(first_segment)?, None)
        };

    let mut socket_addrs = Vec::new();
    if let Some(addr_part) = initial_addr {
        socket_addrs.extend(resolve_socket_addrs(addr_part)?);
    }
    let mut relay_urls = Vec::new();

    for segment in segments {
        let (raw_key, raw_value) = segment.split_once('=').ok_or_else(|| {
            AppError::P2PError(format!(
                "Invalid peer hint segment `{segment}` in `{value}`"
            ))
        })?;

        let key = raw_key.trim().to_ascii_lowercase();
        let field_value = raw_value.trim();
        if field_value.is_empty() {
            return Err(AppError::P2PError(format!(
                "Peer hint segment `{segment}` has empty value in `{value}`"
            )));
        }

        match key.as_str() {
            "addr" | "ip" => {
                socket_addrs.extend(resolve_socket_addrs(field_value)?);
            }
            "relay" | "relay_url" => {
                let relay_url = RelayUrl::from_str(field_value).map_err(|e| {
                    AppError::P2PError(format!("Failed to parse relay URL `{field_value}`: {e}"))
                })?;
                relay_urls.push(relay_url);
            }
            "node" | "node_id" => {
                let parsed_node_id = parse_endpoint_id(field_value)?;
                if parsed_node_id != node_id {
                    return Err(AppError::P2PError(format!(
                        "Conflicting node IDs in peer hint `{value}`"
                    )));
                }
            }
            _ => {
                return Err(AppError::P2PError(format!(
                    "Unsupported peer hint key `{key}` in `{value}`"
                )));
            }
        }
    }

    let node_addr = build_endpoint_addr(node_id, socket_addrs, relay_urls);
    Ok(ParsedPeer { node_id, node_addr })
}

fn prioritize_socket_addrs(addrs: Vec<SocketAddr>) -> Vec<SocketAddr> {
    let mut unique = Vec::new();
    for addr in addrs {
        if !unique.contains(&addr) {
            unique.push(addr);
        }
    }

    let mut ipv4 = Vec::new();
    let mut other = Vec::new();
    for addr in unique {
        if addr.is_ipv4() {
            ipv4.push(addr);
        } else {
            other.push(addr);
        }
    }

    ipv4.extend(other);
    ipv4
}

fn resolve_socket_addrs(addr_part: &str) -> Result<Vec<SocketAddr>, AppError> {
    let trimmed = addr_part.trim();
    if let Ok(socket_addr) = trimmed.parse::<SocketAddr>() {
        return Ok(vec![socket_addr]);
    }

    let (host, port_raw) = trimmed
        .rsplit_once(':')
        .ok_or_else(|| AppError::P2PError(format!("Invalid socket address: {addr_part}")))?;
    let host = host.trim().trim_start_matches('[').trim_end_matches(']');
    if host.is_empty() {
        return Err(AppError::P2PError(format!(
            "Invalid socket address host: {addr_part}"
        )));
    }
    let port: u16 = port_raw
        .trim()
        .parse()
        .map_err(|e| AppError::P2PError(format!("Invalid port `{port_raw}`: {e}")))?;

    if host.eq_ignore_ascii_case("localhost") {
        return Ok(vec![SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), port)]);
    }

    let addrs: Vec<SocketAddr> = (host, port)
        .to_socket_addrs()
        .map_err(|e| AppError::P2PError(format!("Failed to resolve host `{host}`: {e}")))?
        .collect();
    let prioritized = prioritize_socket_addrs(addrs);
    if prioritized.is_empty() {
        return Err(AppError::P2PError(format!(
            "Resolved host `{host}` but no socket addresses were returned"
        )));
    }

    Ok(prioritized)
}

pub fn parse_node_addr(value: &str) -> Result<EndpointAddr, AppError> {
    let trimmed = value.trim();
    if trimmed.contains('|') {
        let parsed = parse_extended_peer_hint(trimmed)?;
        return parsed.node_addr.ok_or_else(|| {
            AppError::P2PError(format!("Peer hint has no address or relay URL: {value}"))
        });
    }

    let (node_part, addr_part) = trimmed
        .split_once('@')
        .ok_or_else(|| AppError::P2PError(format!("Invalid node address format: {value}")))?;

    let node_id = parse_endpoint_id(node_part)?;
    let socket_addrs = resolve_socket_addrs(addr_part)?;
    build_endpoint_addr(node_id, socket_addrs, Vec::new()).ok_or_else(|| {
        AppError::P2PError(format!("Peer hint has no address or relay URL: {value}"))
    })
}

pub fn parse_peer_hint(value: &str) -> Result<ParsedPeer, AppError> {
    let trimmed = value.trim();
    if trimmed.contains('|') {
        return parse_extended_peer_hint(trimmed);
    }

    if trimmed.contains('@') {
        let node_addr = parse_node_addr(trimmed)?;
        Ok(ParsedPeer {
            node_id: node_addr.id,
            node_addr: Some(node_addr),
        })
    } else {
        let node_id = parse_endpoint_id(trimmed)?;
        Ok(ParsedPeer {
            node_id,
            node_addr: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddrV6};
    use std::str::FromStr;

    #[test]
    fn test_parse_node_addr() {
        let addr = parse_node_addr(
            "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef@127.0.0.1:1234",
        )
        .unwrap();
        assert_eq!(addr.ip_addrs().count(), 1);
    }

    #[test]
    fn test_parse_node_addr_localhost() {
        let addr = parse_node_addr(
            "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef@localhost:32145",
        )
        .unwrap();
        let ip_addrs: Vec<_> = addr.ip_addrs().cloned().collect();
        assert_eq!(ip_addrs, vec!["127.0.0.1:32145".parse().unwrap()]);
    }

    #[test]
    fn test_parse_peer_hint_node_id() {
        let node_id = EndpointId::from_str(
            "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
        )
        .unwrap();
        let parsed =
            parse_peer_hint("0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef")
                .unwrap();
        assert_eq!(parsed.node_id, node_id);
        assert!(parsed.node_addr.is_none());
    }

    #[test]
    fn test_parse_peer_hint_with_relay_and_addr() {
        let parsed = parse_peer_hint(
            "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef|relay=https://relay.example|addr=127.0.0.1:4433",
        )
        .unwrap();

        let node_addr = parsed.node_addr.expect("node addr");
        assert_eq!(node_addr.ip_addrs().count(), 1);
        let relay_urls: Vec<_> = node_addr
            .relay_urls()
            .map(|relay_url| relay_url.to_string())
            .collect();
        assert_eq!(relay_urls, vec!["https://relay.example/".to_string()]);
    }

    #[test]
    fn test_parse_peer_hint_with_relay_only() {
        let parsed = parse_peer_hint(
            "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef|relay=https://relay.example",
        )
        .unwrap();

        let node_addr = parsed.node_addr.expect("node addr");
        assert_eq!(node_addr.ip_addrs().count(), 0);
        let relay_urls: Vec<_> = node_addr
            .relay_urls()
            .map(|relay_url| relay_url.to_string())
            .collect();
        assert_eq!(relay_urls, vec!["https://relay.example/".to_string()]);
    }

    #[test]
    fn test_parse_node_addr_allows_extended_hint_with_relay_only() {
        let node_addr = parse_node_addr(
            "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef|relay=https://relay.example",
        )
        .unwrap();
        assert_eq!(node_addr.ip_addrs().count(), 0);
        let relay_urls: Vec<_> = node_addr
            .relay_urls()
            .map(|relay_url| relay_url.to_string())
            .collect();
        assert_eq!(relay_urls, vec!["https://relay.example/".to_string()]);
    }

    #[test]
    fn test_prioritize_socket_addrs_prefers_ipv4() {
        let ipv6 = SocketAddr::V6(SocketAddrV6::new(Ipv6Addr::LOCALHOST, 32145, 0, 0));
        let ipv4 = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 32145);
        let prioritized = prioritize_socket_addrs(vec![ipv6, ipv4]);
        assert_eq!(prioritized, vec![ipv4, ipv6]);
    }
}
