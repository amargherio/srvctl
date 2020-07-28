use trust_dns_resolver::config::*;
use trust_dns_resolver::Resolver;
use url::Url;

use std::fmt::Display;
use std::net::*;

#[derive(Debug)]
pub struct SrvResults {}

impl Display for SrvResults {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "UNIMPLEMENTED FOR STRUCT")
    }
}

/// Resolves an SRV record to its A record components.
/// Returns a struct per underlying A record as well
/// as information regarding the weighting of the record in question.
/// This enables additional processing to correctly address the weighting
/// of each record to ensure proper load balancing across all members of the SRV
/// record in question.
///
/// Based on the implementation of trust-dns, it's recommended to pass in a closing
/// dot after the domain (www.example.com.)
pub async fn resolve_srv(dn: &str) -> anyhow::Result<()> {
    // input validation
    let _val = Url::parse(&dn).unwrap();

    let resolver = Resolver::from_system_conf().unwrap();
    let res = resolver.srv_lookup(dn)?;

    Ok(())
}
