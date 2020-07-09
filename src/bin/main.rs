use anyhow::Result;
use clap::{Clap, App, Arg, ArgMatches};

#[derive(Clap)]
struct Opts {
    #[clap(short, long)]
}

#[tokio::main]
async fn main() -> Result<()> {
    let arg_matches = App::new("srvctl")
        .about("Runs a controller that manages an Endpoint or EndpointSlice representing an SRV DNS record in Kubernetes")
        .arg()
        .arg()
        .arg()
        .get_matches();
}
