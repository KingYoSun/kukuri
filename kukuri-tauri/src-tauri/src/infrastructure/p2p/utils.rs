use crate::shared::error::AppError;
use iroh::{EndpointAddr, EndpointId};
use std::net::SocketAddr;
use std::str::FromStr;

#[derive(Debug, Clone)]
pub struct ParsedPeer {
    pub node_id: EndpointId,
    pub node_addr: Option<EndpointAddr>,
}

pub fn parse_node_addr(value: &str) -> Result<EndpointAddr, AppError> {
    let (node_part, addr_part) = value
        .split_once('@')
        .ok_or_else(|| AppError::P2PError(format!("Invalid node address format: {value}")))?;

    let node_id = EndpointId::from_str(node_part)
        .map_err(|e| AppError::P2PError(format!("Failed to parse node ID: {e}")))?;

    let socket_addr: SocketAddr = addr_part
        .parse()
        .map_err(|e| AppError::P2PError(format!("Failed to parse socket address: {e}")))?;

    Ok(EndpointAddr::new(node_id).with_ip_addr(socket_addr))
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
}
