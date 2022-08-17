use serde::Deserialize;
use std::path::PathBuf;
use url::Url;

#[derive(clap::Args)]
pub struct CmdOpts {
    /// Sets a custom configuration file
    #[clap(long, short, default_value = "spec.toml", value_name = "FILE")]
    pub spec_config: PathBuf,
}

#[derive(Deserialize, Debug)]
pub struct SpecConfig {
    pub server_url: Url,
    pub faucet_pem: PathBuf,
}

pub async fn read_spec_config(path: &PathBuf) -> std::io::Result<SpecConfig> {
    let spec_config_contents = tokio::fs::read(path).await?;
    let spec_config = toml::from_slice(&spec_config_contents)?;
    Ok(spec_config)
}
