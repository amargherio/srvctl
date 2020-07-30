use log::{debug, info, trace, warn};
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
    ipv4_addr: Option<Vec<Ipv4Addr>>,
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
pub async fn resolve_srv(dn: &str) -> anyhow::Result<Vec<SrvResult>> {
    // input validation
    // let _val = Url::parse(&dn).unwrap();
    debug!("Beginning SRV resolution for hostname {}", dn);

    let resolver = TokioAsyncResolver::tokio_from_system_conf().await?;
    let res: trust_dns_resolver::lookup::SrvLookup = resolver.srv_lookup(dn).await?;
    let mut srvres_vec: Vec<SrvResult> = vec![];

    trace!("SRV record resolution results: {:#?}", res);

    // KLUDGE: This should be cleaned up ASAP to better support Ipv6Addr as well as Ipv4Addr
    // and to better utilize the resolution offered by trust-dns-resolver instead
    // of a multi-call ball of yarn that's harder to understand
    info!("Beginning SRV result processing to get IPv4 addresses");
    res.iter().for_each(|srv| {
        trace!("!!! SRV RDATA: {:#?}", srv);
        let srv_res = SrvResult {
            port: srv.port(),
            priority: srv.priority(),
            weight: srv.weight(),
            hostname: srv.target().to_utf8(),
            ipv4_addr: None,
        };
        debug!("New SrvResult struct from SRV RDATA: {}", srv_res);

        srvres_vec.push(srv_res);
    });

    update_ip_addresses_for_results(&srvres_vec, &resolver);

    Ok(srvres_vec)
}

/// Private function called from within the srvctl::dns::resolve_srv function
/// to assist with resolving the A/AAAA records returned down to actual
/// IP addresses.
// TODO: Handle IPv6 gracefully here as well
fn update_ip_addresses_for_results(srv_vec: &Vec<SrvResult>, resolver: &TokioAsyncResolver) {
    for rec in srv_vec.iter_mut() {
        debug!("Performing IPv4 resolution for hostname {}", rec.hostname);
        async {
            if let ipv4_res = resolver.ipv4_lookup(rec.hostname).await {
                match ipv4_res.iter().count() {
                    0 => {
                        warn!("No IP addresses resolved for hostname {}", rec.hostname);
                    }
                    _ => {
                        rec.ipv4_addr = Some(vec![]); // we now have IPs so let's None to Some this
                                                      // iterate over the returned LookupIps and add them to the IP vector
                                                      // for the SrvResult
                        ipv4_res.iter().for_each(|ip| {
                            ip.iter().for_each(|rdata| {
                                rec.ipv4_addr.unwrap().push(rdata.clone());
                            });
                        })
                    }
                }
            }
        };
    }
}
