use std::net::IpAddr;

use edgesentry_rs::{NetworkPolicy, NetworkPolicyError};

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
    p.allow_ip(ip("203.0.113.5"));

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
