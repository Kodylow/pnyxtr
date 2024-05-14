use anyhow::Context;
use clap::Parser;

#[derive(Parser, Debug, Clone)]
#[command(version, author, about)]
/// A NIP 47 tool
pub struct Config {
    #[clap(default_value_t = String::from("keys.json"), long)]
    /// Location of keys file
    pub keys_file: String,
    #[clap(long, required = true)]
    /// Relay to use for communicating
    pub relay: String,
    /// Max invoice payment amount, in satoshis
    #[clap(default_value_t = 100_000, long)]
    pub max_amount: u64,
    /// Max payment amount per day, in satoshis
    #[clap(default_value_t = 100_000, long)]
    pub daily_limit: u64,
    #[clap(long, required = true)]
    /// Datadir for multimint
    pub data_dir: String,
}

fn home_directory() -> Result<String, anyhow::Error> {
    let buf = home::home_dir().context("Failed to get home dir")?;
    let str = format!("{}", buf.display());

    // to be safe remove possible trailing '/' and
    // we can manually add it to paths
    match str.strip_suffix('/') {
        Some(stripped) => Ok(stripped.to_string()),
        None => Ok(str),
    }
}
