use std::{collections::BTreeMap, convert::Infallible, str::FromStr, sync::Arc};

use async_trait::async_trait;
use cucumber::{Parameter, WorldInit};
use many_client::client::ledger::{LedgerClient, Symbol, TokenAmount};
use many_client::ManyClient;
use many_identity::{Address, CoseKeyIdentity};

use crate::{cose::new_identity, opts::SpecConfig};

#[derive(Parameter, Debug, Hash, Ord, PartialOrd, Eq, PartialEq)]
#[param(regex = r"[\w\d]+", name = "identity")]
pub struct IdentityName(String);

impl FromStr for IdentityName {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(IdentityName(s.to_string()))
    }
}

#[derive(Debug, WorldInit)]
pub struct World {
    spec_config: Option<Arc<SpecConfig>>,
    identities: BTreeMap<IdentityName, CoseKeyIdentity>,
    symbols: BTreeMap<String, Symbol>,
    client: Option<ManyClient>,
    ledger_client: Option<LedgerClient>,
}

impl World {
    pub fn client(&self) -> &ManyClient {
        self.client.as_ref().unwrap()
    }

    pub fn ledger_client(&self) -> &LedgerClient {
        self.ledger_client.as_ref().unwrap()
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
        self.ledger_client = Some(LedgerClient::new(self.client().clone()));
        self.symbols = self
            .ledger_client()
            .info()
            .await
            .unwrap()
            .local_names
            .into_iter()
            .map(|(k, v)| (v, k))
            .collect();
    }

    pub fn spec_config(&self) -> &SpecConfig {
        self.spec_config.as_ref().unwrap()
    }

    pub fn symbols(&self) -> &BTreeMap<String, Symbol> {
        &self.symbols
    }

    pub fn symbol(&self, symbol: &str) -> Option<&Symbol> {
        self.symbols().get(symbol)
    }

    pub fn insert_identity(&mut self, id: IdentityName) {
        let identity = new_identity().expect("Should have generated an identity");
        self.identities.insert(id, identity);
    }

    pub fn identity(&self, id: &IdentityName) -> Option<&CoseKeyIdentity> {
        self.identities.get(id)
    }

    pub async fn balance(&self, identity: Address, symbol: Symbol) -> TokenAmount {
        let mut response = self
            .ledger_client()
            .balance(Some(identity), Some(vec![symbol]))
            .await
            .unwrap();
        response
            .balances
            // Remove gets by ownership
            .remove(&symbol)
            .unwrap_or_default()
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
            ledger_client: None,
        })
    }
}

impl Drop for World {
    fn drop(&mut self) {}
}
