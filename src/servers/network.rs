//! Addresses to share with players. Choosing one does not change the server's bind
//! address, open router ports or claim that an internet connection has been tested.
use std::io::Read;
use std::net::{IpAddr, Ipv4Addr};
use std::time::Duration;

/// Minecraft omits its default port; IPv6 literals need brackets with a port.
pub fn address(ip: IpAddr, port: u16) -> String {
    match (ip, port) {
        (IpAddr::V4(ip), 25565) => ip.to_string(),
        (IpAddr::V4(ip), port) => format!("{ip}:{port}"),
        (IpAddr::V6(ip), port) => format!("[{ip}]:{port}"),
    }
}

/// Called only when the owner chooses Internet or asks to refresh it. ipify sees
/// the request's source IP; no account, server settings or game data is sent.
pub fn public_ipv4() -> Result<Ipv4Addr, String> {
    let agent = ureq::AgentBuilder::new()
        .timeout(Duration::from_secs(8))
        .redirects(0)
        .build();
    let response = agent
        .get("https://api.ipify.org")
        .call()
        .map_err(|_| "Public IP lookup failed".to_string())?;
    read_public_ipv4(response.into_reader())
}

fn read_public_ipv4(reader: impl Read) -> Result<Ipv4Addr, String> {
    let mut body = String::new();
    reader
        .take(65)
        .read_to_string(&mut body)
        .map_err(|_| "Could not read the public IP address".to_string())?;
    if body.len() > 64 {
        return Err("Invalid public IP response".into());
    }
    let ip: Ipv4Addr = body
        .trim()
        .parse()
        .map_err(|_| "Invalid public IP response".to_string())?;
    let [a, b, _, _] = ip.octets();
    if ip.is_private()
        || ip.is_loopback()
        || ip.is_link_local()
        || ip.is_unspecified()
        || ip.is_multicast()
        || ip.is_broadcast()
        || ip.is_documentation()
        || a == 0
        || a >= 240
        || (a == 100 && (64..=127).contains(&b))
        || (a == 198 && (18..=19).contains(&b))
    {
        return Err("The lookup did not return a public IPv4 address".into());
    }
    Ok(ip)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn addresses_keep_nondefault_ports_and_bracket_ipv6() {
        assert_eq!(
            address("192.168.2.152".parse().unwrap(), 25565),
            "192.168.2.152"
        );
        assert_eq!(
            address("192.168.2.152".parse().unwrap(), 25566),
            "192.168.2.152:25566"
        );
        assert_eq!(
            address("2001:db8::1".parse().unwrap(), 25565),
            "[2001:db8::1]:25565"
        );
    }

    #[test]
    fn lookup_accepts_plain_ipv4_and_trailing_newlines() {
        assert_eq!(
            read_public_ipv4(&b" 8.8.8.8\n"[..]).unwrap(),
            Ipv4Addr::new(8, 8, 8, 8)
        );
    }

    #[test]
    fn lookup_refuses_unusable_or_malformed_addresses() {
        for body in [
            "",
            "<html>error</html>",
            "8.8.8.8:25565",
            "::1",
            "192.168.2.152",
            "127.0.0.1",
            "100.64.0.1",
            "169.254.1.1",
            "0.0.0.0",
            "255.255.255.255",
            "224.0.0.1",
            "198.51.100.1",
            "198.18.0.1",
        ] {
            assert!(read_public_ipv4(body.as_bytes()).is_err(), "{body}");
        }
        assert!(read_public_ipv4(format!("{}8.8.8.8", " ".repeat(65)).as_bytes()).is_err());
        assert!(read_public_ipv4(&[0xff][..]).is_err());
    }
}
