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
