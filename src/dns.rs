use log::{debug, info, trace};
use trust_dns_resolver::config::*;
use trust_dns_resolver::{lookup_ip::LookupIp, TokioAsyncResolver};
use url::Url;

use std::fmt;
use std::fmt::Display;
use std::net::*;

#[derive(Debug)]
pub struct SrvResult {
    port: u16,
    priority: u16,
    weight: u16,
    hostname: String,
    ipv4_addr: Ipv4Addr,
}

impl Display for SrvResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "&SrvResult {{ port: {}, priority: {}, weight: {}, hostname: {}, ipv4_addr: {} }}",
            self.port, self.priority, self.weight, self.hostname, self.ipv4_addr
        )
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
    // let _val = Url::parse(&dn).unwrap();
    debug!("Beginning SRV resolution for hostname {}", dn);

    let resolver = TokioAsyncResolver::tokio_from_system_conf().await?;
    let res: trust_dns_resolver::lookup::SrvLookup = resolver.srv_lookup(dn).await?;
    let mut srvres_vec: Vec<SrvResult> = vec![];

    trace!("SRV record resolution results: {:#?}", res);

    info!("Beginning SRV result processing to get IPv4 addresses");
    res.iter().for_each(|srv| {
        trace!("!!! SRV RDATA: {:#?}", srv);
        let srv_res = SrvResult {
            port: srv.port(),
            priority: srv.priority(),
            weight: srv.weight(),
            hostname: srv.target().to_utf8(),
            ipv4_addr: Ipv4Addr::new(0, 0, 0, 0),
        };
        debug!("New SrvResult struct from SRV RDATA: {}", srv_res);

        srvres_vec.push(srv_res);
    });

    update_ip_addresses_for_results(&srvres_vec);

    Ok(())
}

fn update_ip_addresses_for_results(srv_vec: &Vec<SrvResult>) {}
