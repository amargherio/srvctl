use log::{debug, info, trace, warn};
use trust_dns_resolver::TokioAsyncResolver;
use url::Url;

use std::fmt;
use std::fmt::Display;
use std::net::Ipv4Addr;

#[derive(Debug)]
pub struct SrvRecord {
    pub port: u16,
    pub priority: u16,
    pub weight: u16,
    pub hostname: String,
    pub ipv4_addr: Option<Vec<Ipv4Addr>>,
}

impl Display for SrvRecord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "&SrvRecord {{ port: {}, priority: {}, weight: {}, hostname: {}, ipv4_addr: {:#?} }}",
            self.port, self.priority, self.weight, self.hostname, self.ipv4_addr,
        )
    }
}

#[derive(Debug)]
pub struct SrvResult {
    pub protocol: Option<String>,
    pub service: Option<String>,
    pub srv_hostname: String,
    pub srv_records: Option<Vec<SrvRecord>>,
}

impl Display for SrvResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "&SrvResult {{ srv_hostname: {}, protocol: {:?}, service: {:?}, srv_records: {:#?} }}",
            self.srv_hostname, self.protocol, self.service, self.srv_records,
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
pub async fn resolve_srv(dn: &str) -> anyhow::Result<SrvResult> {
    // input validation
    // let _val = Url::parse(&dn).unwrap();
    debug!("Beginning SRV resolution for hostname {}", dn);

    let resolver = TokioAsyncResolver::tokio_from_system_conf().await?;
    let res: trust_dns_resolver::lookup::SrvLookup = resolver.srv_lookup(dn).await?;
    let mut srv_recs: Vec<SrvRecord> = vec![];
    let mut srv_res = SrvResult {
        protocol: parse_protocol(dn),
        service: parse_service(dn),
        srv_hostname: String::from(dn),
        srv_records: None,
    };
    trace!("SRV record resolution results: {:#?}", res);

    // KLUDGE: This should be cleaned up ASAP to better support Ipv6Addr as well as Ipv4Addr
    // and to better utilize the resolution offered by trust-dns-resolver instead
    // of a multi-call ball of yarn that's harder to understand
    info!("Beginning SRV result processing to get IPv4 addresses");
    res.iter().for_each(|srv| {
        trace!("!!! SRV RDATA: {:#?}", srv);
        let rec = SrvRecord {
            port: srv.port(),
            priority: srv.priority(),
            weight: srv.weight(),
            hostname: srv.target().to_utf8(),
            ipv4_addr: None,
        };
        debug!("New SrvRecord struct from SRV RDATA: {}", rec);

        srv_recs.push(rec);
    });

    // TODO: extract this into its own function for modularity
    for rec in srv_recs.iter_mut() {
        update_ip_addresses_for_record(rec, &resolver).await;
    }

    srv_res.srv_records = Some(srv_recs);

    trace!("The final SrvResult returned is: {:#?}", srv_res);
    Ok(srv_res)
}

/// Private function called from within the srvctl::dns::resolve_srv function
/// to assist with resolving the A/AAAA records returned down to actual
/// IP addresses.
// TODO: Handle IPv6 gracefully here as well
async fn update_ip_addresses_for_record(rec: &mut SrvRecord, resolver: &TokioAsyncResolver) {
    debug!("Performing IPv4 resolution for hostname {}", rec.hostname);
    trace!("Allocating a new vector for building out the new Ipv4Addr SrvResult vector");
    let mut ipv4_vec: Vec<Ipv4Addr> = vec![];

    // this is required to be an async block due to futures being returned by the DNS lookup
    // calls.
    if let ipv4_res = resolver.ipv4_lookup(rec.hostname.clone()).await.unwrap() {
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
                debug!(
                    "Iterating over the vector of IPs returned and generating the Ipv4Addr vector"
                );
                ipv4_res.iter().for_each(|ip| {
                    trace!("Iterating over the Ipv4Lookup RDATA: {:#?}", ip);
                    ipv4_vec.push(ip.clone());
                });
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

/// Parses the SRV domain to extract the service from the record.
/// Based on SRV record structure, the service is always the first label
/// and is prefixed with an underscore (`_`). This function breaks
/// the URL into labels by splitting on the `.` character and
/// then returns the first label with the prefix stripped.
///
/// Has the potential to return none if the first label is
/// not prefixed with an underscore.
fn parse_service(dn: &str) -> Option<String> {
    // HACK: works for now, maybe try to refactor this into Path::new().components()?
    let vec: Vec<&str> = dn.split(".").collect();

    match vec[0].strip_prefix("_") {
        Some(s) => return Some(String::from(s)),
        None => return None,
    }
}

/// Parses the SRV domain to extract the service from the record.
/// Based on SRV record structure, the service is always the second label
/// and is prefixed with an underscore (`_`). This function breaks
/// the URL into labels by splitting on the `.` character and
/// then returns the second label with the prefix stripped.
///
/// Has the potential to return none if the first label is
/// not prefixed with an underscore.
fn parse_protocol(dn: &str) -> Option<String> {
    // HACK: works for now, maybe try to refactor this into Path::new().components()?
    let vec: Vec<&str> = dn.split(".").collect();

    match vec[1].strip_prefix("_") {
        Some(s) => return Some(String::from(s)),
        None => return None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_protocol_with_correctly_formatted_domain() {
        let dn = "_mongo._tcp.test.database.domain";
        let res = parse_protocol(&dn);

        assert_eq!(res, Some(String::from("tcp")));
    }

    #[test]
    fn test_parse_protocol_with_malformed_domain() {
        let dn = "www.google.com";
        let res = parse_protocol(&dn);

        assert_eq!(res, None);
    }

    #[test]
    fn test_parse_service_with_correctly_formatted_domain() {
        let dn = "_mongo._tcp.test.database.domain";
        let res = parse_service(&dn);

        assert_eq!(res, Some(String::from("mongo")));
    }

    #[test]
    fn test_parse_service_with_malformed_domain() {
        let dn = "www.google.com";
        let res = parse_service(&dn);

        assert_eq!(res, None);
    }
}
