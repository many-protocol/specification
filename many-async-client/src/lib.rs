use many_error::ManyError;
use many_identity::{Address, CoseKeyIdentity};

use coset::{CoseSign1, TaggedCborSerializable};
use many_modules::{
    base::Status,
    ledger::{BalanceArgs, BalanceReturns, InfoReturns, SendArgs},
};
use many_protocol::{
    decode_response_from_cose_sign1, encode_cose_sign1_from_request, RequestMessage,
    RequestMessageBuilder, ResponseMessage,
};
pub use many_types::ledger::Symbol;
use many_types::VecOrSingle;
use minicbor::Encode;
use num_bigint::BigUint;
use reqwest::{IntoUrl, Url};
use std::{collections::BTreeMap, fmt::Formatter};

#[derive(Clone)]
pub struct ManyClient {
    pub id: CoseKeyIdentity,
    pub to: Address,
    url: Url,
}

impl std::fmt::Debug for ManyClient {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ManyClient")
            .field("id", &self.id)
            .field("to", &self.to)
            .field("url", &self.url)
            .finish()
    }
}

impl ManyClient {
    pub fn new<S: IntoUrl>(url: S, to: Address, id: CoseKeyIdentity) -> Result<Self, String> {
        Ok(Self {
            id,
            to,
            url: url.into_url().map_err(|e| format!("{}", e))?,
        })
    }

    pub async fn send_envelope<S: IntoUrl>(
        url: S,
        message: CoseSign1,
    ) -> Result<CoseSign1, ManyError> {
        let bytes = message
            .to_tagged_vec()
            .map_err(|_| ManyError::internal_server_error())?;

        let client = reqwest::Client::new();
        let response = client
            .post(url)
            .body(bytes)
            .send()
            .await
            .map_err(|e| ManyError::unexpected_transport_error(e.to_string()))?;
        let body = response.bytes().await.unwrap();
        let bytes = body.to_vec();
        CoseSign1::from_tagged_slice(&bytes)
            .map_err(|e| ManyError::deserialization_error(e.to_string()))
    }

    pub async fn send_message(
        &self,
        message: RequestMessage,
    ) -> Result<ResponseMessage, ManyError> {
        let cose = encode_cose_sign1_from_request(message, &self.id).unwrap();
        let cose_sign1 = Self::send_envelope(self.url.clone(), cose).await?;

        decode_response_from_cose_sign1(cose_sign1, None).map_err(ManyError::deserialization_error)
    }

    pub async fn call_raw<M>(
        &self,
        method: M,
        argument: &[u8],
    ) -> Result<ResponseMessage, ManyError>
    where
        M: Into<String>,
    {
        let mut nonce = [0u8; 16];
        rand::RngCore::fill_bytes(&mut rand::thread_rng(), &mut nonce);

        let message: RequestMessage = RequestMessageBuilder::default()
            .version(1)
            .from(self.id.identity)
            .to(self.to)
            .method(method.into())
            .data(argument.to_vec())
            .nonce(nonce.to_vec())
            .build()
            .map_err(|_| ManyError::internal_server_error())?;

        self.send_message(message).await
    }

    pub async fn call<M, I>(&self, method: M, argument: I) -> Result<ResponseMessage, ManyError>
    where
        M: Into<String>,
        I: Encode<()>,
    {
        let bytes: Vec<u8> = minicbor::to_vec(argument)
            .map_err(|e| ManyError::serialization_error(e.to_string()))?;

        self.call_raw(method, bytes.as_slice()).await
    }

    pub async fn call_<M, I>(&self, method: M, argument: I) -> Result<Vec<u8>, ManyError>
    where
        M: Into<String>,
        I: Encode<()>,
    {
        self.call(method, argument).await?.data
    }

    pub async fn status(&self) -> Result<Status, ManyError> {
        let response = self.call_("status", ()).await?;

        let status = minicbor::decode(response.as_slice())
            .map_err(|e| ManyError::deserialization_error(e.to_string()))?;
        Ok(status)
    }

    pub async fn symbols(&self) -> Result<BTreeMap<String, Symbol>, ManyError> {
        let response = self.call_("ledger.info", ()).await?;
        let decoded: InfoReturns =
            minicbor::decode(&response).map_err(ManyError::deserialization_error)?;
        Ok(decoded
            .local_names
            .into_iter()
            .map(|(k, v)| (v, k))
            .collect())
    }

    pub async fn balance(&self, account: Address, symbol: Symbol) -> Result<BigUint, ManyError> {
        let symbols = VecOrSingle::from(vec![symbol]);
        let argument = BalanceArgs {
            account: Some(account),
            symbols: Some(symbols),
        };
        let data = self.call_("ledger.balance", argument).await?;
        let response: BalanceReturns =
            minicbor::decode(&data).map_err(ManyError::deserialization_error)?;
        let balance = BigUint::from_bytes_be(
            &response
                .balances
                .get(&symbol)
                .map(|x| x.to_vec())
                .unwrap_or_default(),
        );
        Ok(balance)
    }

    pub async fn send(
        &self,
        from: CoseKeyIdentity,
        to: Address,
        amount: BigUint,
        symbol: Symbol,
    ) -> Result<(), ManyError> {
        let client = ManyClient::new(
            self.url.clone(),
            CoseKeyIdentity::anonymous().identity,
            from.clone(),
        )
        .map_err(|_| ManyError::could_not_route_message())?;
        let argument = SendArgs {
            from: Some(from.identity),
            to,
            amount: amount.into(),
            symbol,
        };
        client.call_("ledger.send", argument).await?;
        Ok(())
    }
}
