use futures::executor::block_on;
use log::{debug, info, trace, warn};
use trust_dns_resolver::{TokioAsyncResolver};
use url::Url;

use std::fmt;
use std::fmt::Display;
use std::net::*;

#[derive(Debug)]
pub struct SrvPort {
    pub port: u16,
    pub service: String,
    pub protocol: String,
}

#[derive(Debug)]
pub struct SrvResult {
    pub srv_port: SrvPort,
    pub port: u16,
    pub priority: u16,
    pub weight: u16,
    pub hostname: String,
    pub ipv4_addr: Option<Vec<Ipv4Addr>>,
}

impl Display for SrvResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "&SrvResult {{ port: {}, priority: {}, weight: {}, hostname: {}, ipv4_addr: {:#?} }}",
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
            srv_port: SrvPort {
                port: srv.port(),
                protocol: String::from("tcp"),
                service: String::from("mongo"),
            },
            port: srv.port(),
            priority: srv.priority(),
            weight: srv.weight(),
            hostname: srv.target().to_utf8(),
            ipv4_addr: None,
        };
        debug!("New SrvResult struct from SRV RDATA: {}", srv_res);

        srvres_vec.push(srv_res);
    });

    // TODO: extract this into its own function for modularity
    for rec in srvres_vec.iter_mut() {
        debug!("Performing IPv4 resolution for hostname {}", rec.hostname);
        trace!("Allocating a new vector for building out the new Ipv4Addr SrvResult vector");
        let mut ipv4_vec: Vec<Ipv4Addr> = vec![];

        // this is required to be an async block due to futures being returned by the DNS lookup
        // calls.
        block_on(async {
            if let ipv4_res = resolver.ipv4_lookup(rec.hostname.clone()).await {
                debug!(
                    "DNS resolution was successful for the hosdtname {}",
                    rec.hostname
                );
                match ipv4_res.iter().count() {
                    0 => {
                        debug!(
                            "No results were found for the hostname supplied: {}",
                            rec.hostname
                        );
                        warn!("No IP addresses resolved for hostname {}", rec.hostname);
                    }
                    _ => {
                        // iterate over the returned LookupIps and add them to the IP vector
                        // for the SrvResult
                        debug!("Iterating over the vector of IPs returned and generating the Ipv4Addr vector");
                        ipv4_res.iter().for_each(|ip| {
                            trace!("Iterating over the Ipv4Lookup {:#?}", ip);
                            ip.iter().for_each(|rdata| {
                                trace!(
                                    "Adding following RDATA to the temporary IPv4 vector: {:#?}",
                                    rdata
                                );
                                ipv4_vec.push(rdata.clone());
                            });
                        })
                    }
                }
            }
            debug!(
                "Updating SrvResult for hostname {} with a vector of {} Ipv4Addr elements",
                rec.hostname,
                ipv4_vec.len()
            );
            rec.ipv4_addr = Some(ipv4_vec.clone());
        });
    }

    trace!("The final SrvResult vector returned is: {:#?}", srvres_vec);
    Ok(srvres_vec)
}

/// Private function called from within the srvctl::dns::resolve_srv function
/// to assist with resolving the A/AAAA records returned down to actual
/// IP addresses.
// TODO: Handle IPv6 gracefully here as well
async fn update_ip_addresses_for_results(
    srv_vec: &mut Vec<SrvResult>,
    resolver: &TokioAsyncResolver,
) {
    for rec in srv_vec.iter_mut() {
        debug!("Performing IPv4 resolution for hostname {}", rec.hostname);
        trace!("Allocating a new vector for building out the new Ipv4Addr SrvResult vector");
        let mut ipv4_vec: Vec<Ipv4Addr> = vec![];

        // this is required to be an async block due to futures being returned by the DNS lookup
        // calls.
        async {
            if let ipv4_res = resolver.ipv4_lookup(rec.hostname.clone()).await {
                debug!(
                    "DNS resolution was successful for the hosdtname {}",
                    rec.hostname
                );
                match ipv4_res.iter().count() {
                    0 => {
                        debug!(
                            "No results were found for the hostname supplied: {}",
                            rec.hostname
                        );
                        warn!("No IP addresses resolved for hostname {}", rec.hostname);
                    }
                    _ => {
                        // iterate over the returned LookupIps and add them to the IP vector
                        // for the SrvResult
                        debug!("Iterating over the vector of IPs returned and generating the Ipv4Addr vector");
                        ipv4_res.iter().for_each(|ip| {
                            trace!("Iterating over the Ipv4Lookup {:#?}", ip);
                            ip.iter().for_each(|rdata| {
                                trace!(
                                    "Adding following RDATA to the temporary IPv4 vector: {:#?}",
                                    rdata
                                );
                                ipv4_vec.push(rdata.clone());
                            });
                        })
                    }
                }
            }
            debug!(
                "Updating SrvResult for hostname {} with a vector of {} Ipv4Addr elements",
                rec.hostname,
                ipv4_vec.len()
            );
            rec.ipv4_addr = Some(ipv4_vec.clone());
        };
    }
}
