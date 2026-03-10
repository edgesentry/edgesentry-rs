use std::net::IpAddr;

use thiserror::Error;

/// A single entry in the allowlist: either an exact IP address or a CIDR block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AllowedSource {
    /// Exact IP address match.
    Ip(IpAddr),
    /// CIDR block (IPv4 or IPv6).
    Cidr { base: IpAddr, prefix_len: u8 },
}

impl AllowedSource {
    /// Returns `true` if `addr` is covered by this entry.
    pub fn contains(&self, addr: IpAddr) -> bool {
        match self {
            AllowedSource::Ip(allowed) => *allowed == addr,
            AllowedSource::Cidr { base, prefix_len } => {
                cidr_contains(*base, *prefix_len, addr)
            }
        }
    }
}

fn cidr_contains(base: IpAddr, prefix_len: u8, addr: IpAddr) -> bool {
    match (base, addr) {
        (IpAddr::V4(base_v4), IpAddr::V4(addr_v4)) => {
            if prefix_len == 0 {
                return true;
            }
            if prefix_len > 32 {
                return false;
            }
            let shift = 32 - prefix_len as u32;
            let base_bits = u32::from(base_v4) >> shift;
            let addr_bits = u32::from(addr_v4) >> shift;
            base_bits == addr_bits
        }
        (IpAddr::V6(base_v6), IpAddr::V6(addr_v6)) => {
            if prefix_len == 0 {
                return true;
            }
            if prefix_len > 128 {
                return false;
            }
            let shift = 128 - prefix_len as u32;
            let base_bits = u128::from(base_v6) >> shift;
            let addr_bits = u128::from(addr_v6) >> shift;
            base_bits == addr_bits
        }
        // Mismatched families never match.
        _ => false,
    }
}

/// Errors produced by [`NetworkPolicy`].
#[derive(Debug, Error, PartialEq, Eq)]
pub enum NetworkPolicyError {
    /// Source address is not in the allowlist.
    #[error("source {addr} is not in the allowlist")]
    Denied { addr: IpAddr },
    /// The supplied CIDR string could not be parsed.
    #[error("invalid CIDR '{0}': expected <addr>/<prefix_len>")]
    InvalidCidr(String),
}

/// Deny-by-default IP/CIDR allowlist for ingest endpoints.
///
/// # Usage
///
/// Build a policy with [`NetworkPolicy::new`], populate it with
/// [`allow_ip`](Self::allow_ip) / [`allow_cidr`](Self::allow_cidr), then call
/// [`check`](Self::check) with the source address of each incoming connection
/// **before** passing the payload to [`IngestService`](super::IngestService).
///
/// ```rust
/// use std::net::IpAddr;
/// use edgesentry_rs::NetworkPolicy;
///
/// let mut policy = NetworkPolicy::new();
/// policy.allow_cidr("10.0.0.0/8").unwrap();
///
/// let trusted: IpAddr = "10.1.2.3".parse().unwrap();
/// assert!(policy.check(trusted).is_ok());
///
/// let untrusted: IpAddr = "192.168.1.1".parse().unwrap();
/// assert!(policy.check(untrusted).is_err());
/// ```
#[derive(Debug, Default, Clone)]
pub struct NetworkPolicy {
    allowed: Vec<AllowedSource>,
}

impl NetworkPolicy {
    /// Create an empty policy (all sources denied until rules are added).
    pub fn new() -> Self {
        Self::default()
    }

    /// Permit a single IP address.
    pub fn allow_ip(&mut self, addr: IpAddr) -> &mut Self {
        self.allowed.push(AllowedSource::Ip(addr));
        self
    }

    /// Permit all addresses within a CIDR block, e.g. `"10.0.0.0/8"` or `"fd00::/8"`.
    pub fn allow_cidr(&mut self, cidr: &str) -> Result<&mut Self, NetworkPolicyError> {
        let (base, prefix_len) = parse_cidr(cidr)?;
        self.allowed.push(AllowedSource::Cidr { base, prefix_len });
        Ok(self)
    }

    /// Returns `Ok(())` if `source` is covered by at least one allowlist entry,
    /// or `Err(NetworkPolicyError::Denied)` if not.
    pub fn check(&self, source: IpAddr) -> Result<(), NetworkPolicyError> {
        if self.allowed.iter().any(|e| e.contains(source)) {
            Ok(())
        } else {
            Err(NetworkPolicyError::Denied { addr: source })
        }
    }

    /// Returns the list of configured allowlist entries.
    pub fn entries(&self) -> &[AllowedSource] {
        &self.allowed
    }
}

fn parse_cidr(cidr: &str) -> Result<(IpAddr, u8), NetworkPolicyError> {
    let err = || NetworkPolicyError::InvalidCidr(cidr.to_string());

    let (addr_str, prefix_str) = cidr.split_once('/').ok_or_else(err)?;
    let prefix_len: u8 = prefix_str.parse().map_err(|_| err())?;
    let base: IpAddr = addr_str.parse().map_err(|_| err())?;

    let max_prefix = match base {
        IpAddr::V4(_) => 32,
        IpAddr::V6(_) => 128,
    };
    if prefix_len > max_prefix {
        return Err(err());
    }

    Ok((base, prefix_len))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ip(s: &str) -> IpAddr {
        s.parse().unwrap()
    }

    // ── exact IP ────────────────────────────────────────────────────────────

    #[test]
    fn allow_exact_ipv4_permits_that_ip() {
        let mut p = NetworkPolicy::new();
        p.allow_ip(ip("192.168.1.10"));
        assert!(p.check(ip("192.168.1.10")).is_ok());
    }

    #[test]
    fn allow_exact_ipv4_denies_other_ip() {
        let mut p = NetworkPolicy::new();
        p.allow_ip(ip("192.168.1.10"));
        let err = p.check(ip("192.168.1.11")).unwrap_err();
        assert_eq!(err, NetworkPolicyError::Denied { addr: ip("192.168.1.11") });
    }

    #[test]
    fn empty_policy_denies_everything() {
        let p = NetworkPolicy::new();
        assert!(p.check(ip("127.0.0.1")).is_err());
        assert!(p.check(ip("10.0.0.1")).is_err());
    }

    #[test]
    fn multiple_exact_ips_any_match_allowed() {
        let mut p = NetworkPolicy::new();
        p.allow_ip(ip("10.0.0.1"));
        p.allow_ip(ip("10.0.0.2"));
        assert!(p.check(ip("10.0.0.1")).is_ok());
        assert!(p.check(ip("10.0.0.2")).is_ok());
        assert!(p.check(ip("10.0.0.3")).is_err());
    }

    // ── CIDR IPv4 ───────────────────────────────────────────────────────────

    #[test]
    fn cidr_ipv4_24_allows_addresses_in_subnet() {
        let mut p = NetworkPolicy::new();
        p.allow_cidr("10.1.2.0/24").unwrap();
        assert!(p.check(ip("10.1.2.1")).is_ok());
        assert!(p.check(ip("10.1.2.254")).is_ok());
    }

    #[test]
    fn cidr_ipv4_24_denies_outside_subnet() {
        let mut p = NetworkPolicy::new();
        p.allow_cidr("10.1.2.0/24").unwrap();
        assert!(p.check(ip("10.1.3.1")).is_err());
        assert!(p.check(ip("10.2.2.1")).is_err());
    }

    #[test]
    fn cidr_ipv4_8_covers_full_class_a() {
        let mut p = NetworkPolicy::new();
        p.allow_cidr("10.0.0.0/8").unwrap();
        assert!(p.check(ip("10.0.0.1")).is_ok());
        assert!(p.check(ip("10.255.255.255")).is_ok());
        assert!(p.check(ip("11.0.0.1")).is_err());
    }

    #[test]
    fn cidr_ipv4_slash_32_is_exact_match() {
        let mut p = NetworkPolicy::new();
        p.allow_cidr("192.168.50.5/32").unwrap();
        assert!(p.check(ip("192.168.50.5")).is_ok());
        assert!(p.check(ip("192.168.50.6")).is_err());
    }

    #[test]
    fn cidr_ipv4_slash_0_allows_all() {
        let mut p = NetworkPolicy::new();
        p.allow_cidr("0.0.0.0/0").unwrap();
        assert!(p.check(ip("1.2.3.4")).is_ok());
        assert!(p.check(ip("255.255.255.255")).is_ok());
    }

    // ── CIDR IPv6 ───────────────────────────────────────────────────────────

    #[test]
    fn cidr_ipv6_allows_address_in_block() {
        let mut p = NetworkPolicy::new();
        p.allow_cidr("fd00::/8").unwrap();
        assert!(p.check(ip("fd00::1")).is_ok());
        assert!(p.check(ip("fdff::1")).is_ok());
    }

    #[test]
    fn cidr_ipv6_denies_outside_block() {
        let mut p = NetworkPolicy::new();
        p.allow_cidr("fd00::/8").unwrap();
        assert!(p.check(ip("fe80::1")).is_err());
    }

    #[test]
    fn ipv4_cidr_does_not_match_ipv6_address() {
        let mut p = NetworkPolicy::new();
        p.allow_cidr("10.0.0.0/8").unwrap();
        // IPv6-mapped IPv4 is a different type in Rust's IpAddr enum.
        assert!(p.check(ip("::ffff:10.0.0.1")).is_err());
    }

    // ── parse errors ────────────────────────────────────────────────────────

    #[test]
    fn allow_cidr_rejects_missing_slash() {
        let mut p = NetworkPolicy::new();
        let err = p.allow_cidr("10.0.0.0").unwrap_err();
        assert!(matches!(err, NetworkPolicyError::InvalidCidr(_)));
    }

    #[test]
    fn allow_cidr_rejects_non_numeric_prefix() {
        let mut p = NetworkPolicy::new();
        let err = p.allow_cidr("10.0.0.0/abc").unwrap_err();
        assert!(matches!(err, NetworkPolicyError::InvalidCidr(_)));
    }

    #[test]
    fn allow_cidr_rejects_prefix_out_of_range_ipv4() {
        let mut p = NetworkPolicy::new();
        let err = p.allow_cidr("10.0.0.0/33").unwrap_err();
        assert!(matches!(err, NetworkPolicyError::InvalidCidr(_)));
    }

    #[test]
    fn allow_cidr_rejects_invalid_ip() {
        let mut p = NetworkPolicy::new();
        let err = p.allow_cidr("999.0.0.0/24").unwrap_err();
        assert!(matches!(err, NetworkPolicyError::InvalidCidr(_)));
    }

    // ── mixed rules ─────────────────────────────────────────────────────────

    #[test]
    fn mixed_ip_and_cidr_rules_work_together() {
        let mut p = NetworkPolicy::new();
        p.allow_cidr("10.0.0.0/8").unwrap();
        p.allow_ip(ip("203.0.113.5")); // single external IP

        assert!(p.check(ip("10.42.0.1")).is_ok());
        assert!(p.check(ip("203.0.113.5")).is_ok());
        assert!(p.check(ip("203.0.113.6")).is_err());
        assert!(p.check(ip("192.168.1.1")).is_err());
    }

    // ── loopback / localhost ─────────────────────────────────────────────────

    #[test]
    fn loopback_is_denied_by_default_unless_explicitly_allowed() {
        let p = NetworkPolicy::new();
        assert!(p.check(ip("127.0.0.1")).is_err());
    }

    #[test]
    fn loopback_allowed_when_explicitly_added() {
        let mut p = NetworkPolicy::new();
        p.allow_cidr("127.0.0.0/8").unwrap();
        assert!(p.check(ip("127.0.0.1")).is_ok());
    }

    // ── entries accessor ─────────────────────────────────────────────────────

    #[test]
    fn entries_reflects_added_rules() {
        let mut p = NetworkPolicy::new();
        p.allow_ip(ip("1.2.3.4"));
        p.allow_cidr("10.0.0.0/8").unwrap();
        assert_eq!(p.entries().len(), 2);
    }
}
