use anyhow::Result;
use trust_dns_resolver::Resolver;
use trust_dns_resolver::config::*;

use std::net::*;

/// Resolves an SRV record to its A record components.
/// Returns a struct per underlying A record as well
/// as information regarding the weighting of the record in question.
/// This enables additional processing to correctly address the weighting
/// of each record to ensure proper load balancing across all members of the SRV
/// record in question.
///
/// Based on the implementation of trust-dns, it's recommended to pass in a closing
/// dot after the domain (www.example.com.)
pub async fn resolve_srv(dn: &str) -> Result<LookupIp> {
    // input validation
    let val = Url::parse(&str).unwrap();

    let resolver = Resolver::from_system_conf().unwrap();
    let ips = resolver.lookup_ip(&str).unwrap();

    ips
}