use crate::shared::error::AppError;
use iroh::{EndpointAddr, EndpointId};
use std::net::{IpAddr, Ipv4Addr, SocketAddr, ToSocketAddrs};
use std::str::FromStr;

#[derive(Debug, Clone)]
pub struct ParsedPeer {
    pub node_id: EndpointId,
    pub node_addr: Option<EndpointAddr>,
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
    let (node_part, addr_part) = value
        .split_once('@')
        .ok_or_else(|| AppError::P2PError(format!("Invalid node address format: {value}")))?;

    let node_id = EndpointId::from_str(node_part)
        .map_err(|e| AppError::P2PError(format!("Failed to parse node ID: {e}")))?;

    let socket_addrs = resolve_socket_addrs(addr_part)?;
    let mut endpoint_addr = EndpointAddr::new(node_id);
    for socket_addr in socket_addrs {
        endpoint_addr = endpoint_addr.with_ip_addr(socket_addr);
    }

    Ok(endpoint_addr)
}

pub fn parse_peer_hint(value: &str) -> Result<ParsedPeer, AppError> {
    if value.contains('@') {
        let node_addr = parse_node_addr(value)?;
        Ok(ParsedPeer {
            node_id: node_addr.id,
            node_addr: Some(node_addr),
        })
    } else {
        let node_id = EndpointId::from_str(value)
            .map_err(|e| AppError::P2PError(format!("Failed to parse node ID: {e}")))?;
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
    fn test_prioritize_socket_addrs_prefers_ipv4() {
        let ipv6 = SocketAddr::V6(SocketAddrV6::new(Ipv6Addr::LOCALHOST, 32145, 0, 0));
        let ipv4 = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 32145);
        let prioritized = prioritize_socket_addrs(vec![ipv6, ipv4]);
        assert_eq!(prioritized, vec![ipv4, ipv6]);
    }
}
