use many_identity::CoseKeyIdentity;
use serde::{de::Visitor, Deserialize, Deserializer};
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
    #[serde(rename = "faucet_pem", deserialize_with = "deserialize_identity")]
    pub faucet_identity: CoseKeyIdentity,
}

fn deserialize_identity<'de, D>(d: D) -> Result<CoseKeyIdentity, D::Error>
where
    D: Deserializer<'de>,
{
    struct InternalVisitor;
    impl<'de> Visitor<'de> for InternalVisitor {
        type Value = CoseKeyIdentity;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("Expecting a path to a pem file")
        }

        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            let pem = std::fs::read_to_string(v).map_err(E::custom)?;
            CoseKeyIdentity::from_pem(&pem).map_err(E::custom)
        }
    }
    d.deserialize_any(InternalVisitor)
}

pub async fn read_spec_config(path: &PathBuf) -> std::io::Result<SpecConfig> {
    let spec_config_contents = tokio::fs::read(path).await?;
    let spec_config = toml::from_slice(&spec_config_contents)?;
    Ok(spec_config)
}
