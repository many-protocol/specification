use std::{collections::BTreeMap, convert::Infallible, sync::Arc};

use async_trait::async_trait;
use cucumber::WorldInit;
use many_client::client::base::BaseClient;
use many_client::client::ledger::{BalanceArgs, LedgerClient, Symbol, TokenAmount};
use many_client::ManyClient;
use many_identity::{Address, AnonymousIdentity, Identity};
use many_identity_dsa::CoseKeyIdentity;

use crate::params::Identifier;
use crate::{cose::new_identity, opts::SpecConfig};

#[derive(Debug, WorldInit)]
pub struct World {
    spec_config: Option<Arc<SpecConfig>>,
    identities: BTreeMap<Identifier, CoseKeyIdentity>,
    symbols: BTreeMap<String, Symbol>,
    ledger_clients: BTreeMap<Address, LedgerClient<CoseKeyIdentity>>,
    base_client: Option<BaseClient<CoseKeyIdentity>>,
}

impl World {
    pub fn faucet_ledger_client(&self) -> &LedgerClient<CoseKeyIdentity> {
        self.ledger_client(self.spec_config().faucet_identity.address())
    }

    pub fn base_client(&self) -> &BaseClient<CoseKeyIdentity> {
        self.base_client.as_ref().unwrap()
    }

    pub async fn init_config(&mut self, spec_config: Arc<SpecConfig>) {
        self.spec_config = Some(spec_config);

        let faucet_identity = self.spec_config().faucet_identity.clone();

        let faucet_client = ManyClient::new(
            self.spec_config().server_url.clone(),
            AnonymousIdentity.address(),
            faucet_identity.clone(),
        )
        .unwrap();

        self.base_client = Some(BaseClient::new(faucet_client.clone()));
        self.ledger_clients
            .insert(faucet_identity.address(), LedgerClient::new(faucet_client));

        self.symbols = self
            .faucet_ledger_client()
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

    pub fn insert_identity(&mut self, id: Identifier) {
        let identity = new_identity();
        self.identities.insert(id, identity.clone());
        let many_client = ManyClient::new(
            self.spec_config().server_url.clone(),
            AnonymousIdentity.address(),
            identity.clone(),
        )
        .unwrap();
        let ledger_client = LedgerClient::new(many_client);
        self.ledger_clients
            .insert(identity.address(), ledger_client);
    }

    pub fn identity(&self, id: &Identifier) -> Option<&CoseKeyIdentity> {
        self.identities.get(id)
    }

    pub fn ledger_client(&self, id: Address) -> &LedgerClient<CoseKeyIdentity> {
        self.ledger_clients.get(&id).unwrap()
    }

    pub async fn balance(&self, identity: Address, symbol: Symbol) -> TokenAmount {
        let mut response = self
            .ledger_client(identity)
            .balance(BalanceArgs {
                account: Some(identity),
                symbols: Some(vec![symbol].into()),
            })
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
            ledger_clients: BTreeMap::new(),
            base_client: None,
        })
    }
}

impl Drop for World {
    fn drop(&mut self) {}
}
