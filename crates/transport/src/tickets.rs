use std::net::{IpAddr, Ipv4Addr, SocketAddr, ToSocketAddrs};
use std::str::FromStr;

use anyhow::{Context, Result, anyhow};
use iroh::{EndpointAddr, EndpointId, RelayUrl};

use crate::config::{SeedPeer, TransportNetworkConfig};

impl SeedPeer {
    pub fn to_endpoint_addr(&self) -> Result<EndpointAddr> {
        self.to_endpoint_addr_with_relays(&[])
    }

    pub fn to_endpoint_addr_with_relays(&self, relay_urls: &[RelayUrl]) -> Result<EndpointAddr> {
        let endpoint_id = EndpointId::from_str(self.endpoint_id.trim())
            .with_context(|| format!("invalid seed endpoint id `{}`", self.endpoint_id))?;
        let mut endpoint_addr = match self.addr_hint.as_deref() {
            Some(addr_hint) => {
                let socket_addrs = resolve_socket_addrs(addr_hint)?;
                build_endpoint_addr(endpoint_id, socket_addrs).ok_or_else(|| {
                    anyhow!("seed peer must resolve to at least one socket address")
                })?
            }
            None => endpoint_addr_with_relays(endpoint_id, relay_urls),
        };
        for relay_url in relay_urls {
            endpoint_addr = endpoint_addr.with_relay_url(relay_url.clone());
        }
        Ok(endpoint_addr)
    }

    pub fn display(&self) -> String {
        match self.addr_hint.as_deref() {
            Some(addr_hint) => format!("{}@{}", self.endpoint_id, addr_hint.trim()),
            None => self.endpoint_id.clone(),
        }
    }
}

pub fn encode_endpoint_ticket(
    endpoint_addr: &EndpointAddr,
    config: &TransportNetworkConfig,
) -> Result<String> {
    let advertised_port = config
        .advertised_port
        .or_else(|| {
            endpoint_addr
                .ip_addrs()
                .find(|addr| addr.port() != 0)
                .map(|addr| addr.port())
        })
        .or_else(|| match config.bind_addr {
            SocketAddr::V4(addr) if addr.port() != 0 => Some(addr.port()),
            SocketAddr::V6(addr) if addr.port() != 0 => Some(addr.port()),
            _ => None,
        })
        .ok_or_else(|| anyhow!("could not determine advertised port"))?;
    let advertised_host = config
        .advertised_host
        .clone()
        .or_else(|| {
            endpoint_addr
                .ip_addrs()
                .filter(|addr| is_reachable_ip(addr.ip()))
                .map(|addr| addr.ip().to_string())
                .next()
        })
        .or_else(|| match config.bind_addr.ip() {
            ip if is_reachable_ip(ip) => Some(ip.to_string()),
            IpAddr::V4(ip) if ip.is_loopback() => Some(ip.to_string()),
            IpAddr::V6(ip) if ip.is_loopback() => Some(ip.to_string()),
            _ => None,
        })
        .ok_or_else(|| anyhow!("could not determine advertised host"))?;

    Ok(format!(
        "{}@{}",
        endpoint_addr.id,
        format_host_port(&advertised_host, advertised_port)
    ))
}

pub fn parse_seed_peer(value: &str) -> Result<SeedPeer> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        anyhow::bail!("seed peer must not be empty");
    }
    let seed = if let Some((endpoint_id, addr_hint)) = trimmed.split_once('@') {
        SeedPeer {
            endpoint_id: endpoint_id.trim().to_string(),
            addr_hint: Some(addr_hint.trim().to_string()),
        }
    } else {
        SeedPeer {
            endpoint_id: trimmed.to_string(),
            addr_hint: None,
        }
    };
    let _ = seed.to_endpoint_addr()?;
    Ok(seed)
}

pub fn parse_endpoint_ticket(ticket: &str) -> Result<EndpointAddr> {
    let (node_id, socket_addr) = ticket
        .split_once('@')
        .ok_or_else(|| anyhow!("ticket must be formatted as <node_id>@<host:port>"))?;
    let endpoint_id = EndpointId::from_str(node_id).context("invalid endpoint id")?;
    let socket_addrs = resolve_socket_addrs(socket_addr)?;
    build_endpoint_addr(endpoint_id, socket_addrs)
        .ok_or_else(|| anyhow!("ticket must resolve to at least one socket address"))
}

pub(crate) fn ticket_network_config(
    endpoint_addr: &EndpointAddr,
    bound_sockets: &[SocketAddr],
    config: &TransportNetworkConfig,
) -> TransportNetworkConfig {
    let advertised_host = config.advertised_host.clone().or_else(|| {
        bound_sockets
            .iter()
            .find(|addr| is_reachable_ip(addr.ip()) || addr.ip().is_loopback())
            .map(|addr| addr.ip().to_string())
    });
    let advertised_port = config.advertised_port.or_else(|| {
        bound_sockets
            .iter()
            .find(|addr| addr.port() != 0)
            .map(|addr| addr.port())
    });

    if advertised_host.is_none() && advertised_port.is_none() {
        return config.clone();
    }

    TransportNetworkConfig {
        bind_addr: config.bind_addr,
        advertised_host: advertised_host.or_else(|| {
            endpoint_addr
                .ip_addrs()
                .find(|addr| is_reachable_ip(addr.ip()) || addr.ip().is_loopback())
                .map(|addr| addr.ip().to_string())
        }),
        advertised_port: advertised_port.or_else(|| {
            endpoint_addr
                .ip_addrs()
                .find(|addr| addr.port() != 0)
                .map(|addr| addr.port())
        }),
    }
}

pub(crate) fn build_endpoint_addr(
    endpoint_id: EndpointId,
    socket_addrs: Vec<SocketAddr>,
) -> Option<EndpointAddr> {
    if socket_addrs.is_empty() {
        return None;
    }

    let mut endpoint_addr = EndpointAddr::new(endpoint_id);
    for socket_addr in socket_addrs {
        endpoint_addr = endpoint_addr.with_ip_addr(socket_addr);
    }
    Some(endpoint_addr)
}

pub(crate) fn endpoint_addr_with_relays(
    endpoint_id: EndpointId,
    relay_urls: &[RelayUrl],
) -> EndpointAddr {
    let mut endpoint_addr = EndpointAddr::new(endpoint_id);
    for relay_url in relay_urls {
        endpoint_addr = endpoint_addr.with_relay_url(relay_url.clone());
    }
    endpoint_addr
}

pub(crate) fn resolve_socket_addrs(value: &str) -> Result<Vec<SocketAddr>> {
    let trimmed = value.trim();
    if let Ok(socket_addr) = trimmed.parse::<SocketAddr>() {
        return Ok(vec![socket_addr]);
    }

    let (host, port_raw) = trimmed
        .rsplit_once(':')
        .ok_or_else(|| anyhow!("invalid socket address: {value}"))?;
    let host = host.trim().trim_start_matches('[').trim_end_matches(']');
    let port = port_raw
        .trim()
        .parse::<u16>()
        .with_context(|| format!("invalid port in `{value}`"))?;

    let addrs = if host.eq_ignore_ascii_case("localhost") {
        vec![SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), port)]
    } else {
        (host, port)
            .to_socket_addrs()
            .with_context(|| format!("failed to resolve host `{host}`"))?
            .collect::<Vec<_>>()
    };

    Ok(prioritize_socket_addrs(addrs))
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

fn format_host_port(host: &str, port: u16) -> String {
    let trimmed = host.trim().trim_start_matches('[').trim_end_matches(']');
    if trimmed.contains(':') {
        format!("[{trimmed}]:{port}")
    } else {
        format!("{trimmed}:{port}")
    }
}

fn is_reachable_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(ip) => !ip.is_unspecified() && !ip.is_loopback(),
        IpAddr::V6(ip) => !ip.is_unspecified() && !ip.is_loopback(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ticket_roundtrip() {
        let ticket =
            "f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0@127.0.0.1:4444";
        let parsed = parse_endpoint_ticket(ticket).expect("ticket must parse");
        assert_eq!(
            parsed.id.to_string(),
            "f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0"
        );
        assert_eq!(
            parsed.ip_addrs().next().copied(),
            Some("127.0.0.1:4444".parse().expect("socket addr"))
        );
    }

    #[test]
    fn encode_ticket_prefers_explicit_advertised_host() {
        let endpoint_id = EndpointId::from_str(
            "f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0",
        )
        .expect("endpoint id");
        let endpoint_addr = EndpointAddr::new(endpoint_id)
            .with_ip_addr("0.0.0.0:40123".parse().expect("socket addr"));
        let config = TransportNetworkConfig {
            bind_addr: "0.0.0.0:40123".parse().expect("bind addr"),
            advertised_host: Some("192.168.10.5".into()),
            advertised_port: Some(40123),
        };

        let ticket = encode_endpoint_ticket(&endpoint_addr, &config).expect("ticket");
        assert_eq!(
            ticket,
            "f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0@192.168.10.5:40123"
        );
    }

    #[test]
    fn encode_ticket_ignores_zero_port_from_endpoint_addr() {
        let endpoint_id = EndpointId::from_str(
            "f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0",
        )
        .expect("endpoint id");
        let endpoint_addr =
            EndpointAddr::new(endpoint_id).with_ip_addr("0.0.0.0:0".parse().expect("socket addr"));
        let config = TransportNetworkConfig {
            bind_addr: "127.0.0.1:40123".parse().expect("bind addr"),
            advertised_host: None,
            advertised_port: None,
        };

        let ticket = encode_endpoint_ticket(&endpoint_addr, &config).expect("ticket");

        assert_eq!(
            ticket,
            "f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0@127.0.0.1:40123"
        );
    }

    #[test]
    fn ticket_network_config_uses_bound_loopback_socket_for_port_zero_bind() {
        let endpoint_id = EndpointId::from_str(
            "f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0",
        )
        .expect("endpoint id");
        let endpoint_addr =
            EndpointAddr::new(endpoint_id).with_ip_addr("0.0.0.0:0".parse().expect("socket addr"));
        let config = TransportNetworkConfig::loopback();

        let resolved = ticket_network_config(
            &endpoint_addr,
            &["127.0.0.1:40123".parse().expect("bound socket")],
            &config,
        );

        assert_eq!(resolved.advertised_host.as_deref(), Some("127.0.0.1"));
        assert_eq!(resolved.advertised_port, Some(40123));
    }

    #[test]
    fn parse_ticket_resolves_localhost_hostname() {
        let parsed = parse_endpoint_ticket(
            "f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0@localhost:40123",
        )
        .expect("ticket");
        assert_eq!(
            parsed.ip_addrs().next().copied(),
            Some("127.0.0.1:40123".parse().expect("socket addr"))
        );
    }
}
