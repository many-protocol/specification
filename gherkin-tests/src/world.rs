use std::{collections::BTreeMap, convert::Infallible, str::FromStr, sync::Arc};

use async_trait::async_trait;
use cucumber::{Parameter, WorldInit};
use many_async_client::{ManyClient, Symbol};
use many_identity::CoseKeyIdentity;

use crate::{cose::new_identity, opts::SpecConfig};

#[derive(Parameter, Debug, Hash, Ord, PartialOrd, Eq, PartialEq)]
#[param(regex = r"[\w\d]+", name = "identity")]
pub struct IdentityName([u8; 4]);

impl FromStr for IdentityName {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let bytes = s.as_bytes();
        let len = bytes.len();
        if len > 4 {
            return Err("Identity name is too big".into());
        }
        let v: Vec<u8> = std::iter::repeat(&0u8)
            .take(4 - len)
            .chain(bytes)
            .copied()
            .collect();
        Ok(IdentityName([v[0], v[1], v[2], v[3]]))
    }
}

impl From<&IdentityName> for u32 {
    fn from(name: &IdentityName) -> u32 {
        u32::from_be_bytes(name.0)
    }
}

#[derive(Debug, WorldInit)]
pub struct World {
    spec_config: Option<Arc<SpecConfig>>,
    identities: BTreeMap<IdentityName, CoseKeyIdentity>,
    symbols: BTreeMap<String, Symbol>,
    client: Option<ManyClient>,
}

impl World {
    pub fn client(&mut self) -> &ManyClient {
        self.client.as_ref().unwrap()
    }

    pub async fn init_config(&mut self, spec_config: Arc<SpecConfig>) {
        self.client = Some(
            ManyClient::new(
                spec_config.server_url.clone(),
                spec_config.ledger_identity.identity,
                spec_config.ledger_identity.clone(),
            )
            .unwrap(),
        );
        self.spec_config = Some(spec_config);
        self.symbols = self.client().symbols().await.unwrap();
    }

    pub fn spec_config(&self) -> &SpecConfig {
        self.spec_config.as_ref().unwrap()
    }

    pub fn symbols(&mut self) -> &BTreeMap<String, Symbol> {
        &self.symbols
    }

    pub fn symbol(&mut self, symbol: &str) -> Option<&Symbol> {
        self.symbols().get(symbol)
    }

    pub fn insert_identity(&mut self, id: IdentityName) {
        let identity = new_identity(&id).expect("Should have generated an identity");
        self.identities.insert(id, identity);
    }

    pub fn identity(&self, id: &IdentityName) -> Option<&CoseKeyIdentity> {
        self.identities.get(id)
    }
}

#[async_trait(?Send)]
impl cucumber::World for World {
    type Error = Infallible;

    async fn new() -> Result<Self, Self::Error> {
        Ok(World {
            spec_config: None,
            identities: BTreeMap::new(),
            symbols: BTreeMap::new(),
            client: None,
        })
    }
}

impl Drop for World {
    fn drop(&mut self) {}
}
