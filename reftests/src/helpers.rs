use crate::TestConfig;
use ciborium::value::{Integer, Value};
use coset::{AsCborValue, CborSerializable, CoseSign1, Header, TaggedCborSerializable};
use ed25519_dalek::{PublicKey, Signer};
use many_types::attributes::AttributeId;
use pkcs8::der::Document;
use rand::SeedableRng;
use reqwest::StatusCode;
use sha3::{Digest, Sha3_224};
use std::{
    collections::BTreeMap,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

#[derive(Default)]
pub struct MessageKey {
    pub ed25519_key: Option<ed25519_dalek::Keypair>,
    pub kid: Option<Vec<u8>>,
    pub protected: Option<Header>,
}

#[derive(PartialEq)]
pub enum KeyType {
    KeySeed(u8),
    PrivateKey(String),
}

pub async fn has_attributes(
    attrs: Option<Vec<AttributeId>>,
    config: &TestConfig,
) -> Result<(), String> {
    if attrs.is_some() {
        for a in attrs.unwrap_or_default().into_iter() {
            if !has_attribute(a, &config).await {
                return Err(format!("Server does not support attribute: {}", a));
            }
        }
    }
    Ok(())
}

pub async fn has_attribute(attr: AttributeId, config: &TestConfig) -> bool {
    let result = send(config, anonymous_message("status", "null")).await;

    let payload = result.payload.expect("No payload from status");

    let response: BTreeMap<u8, Value> =
        ciborium::de::from_reader(payload.as_slice()).expect("Invalid payload.");
    let response_payload = response
        .get(&4)
        .expect("Response return value was not found")
        .as_bytes()
        .expect("Response return value was expected to be Bytes");

    let status: Value =
        ciborium::de::from_reader(response_payload.as_slice()).expect("Invalid response payload");

    let attrs = if let Value::Map(m) = status {
        // Find key 4 for attributes.
        m.into_iter()
            .find(|(key, _)| key.as_integer().map_or(false, |i| i == Integer::from(4)))
    } else {
        panic!("Status was not a map.")
    };

    let attr = Integer::from(attr);
    match attrs {
        Some((_, Value::Array(attr_list))) => attr_list.iter().any(|v| match v {
            Value::Integer(i) => i == &attr,
            Value::Array(v) => {
                if let Some(Value::Integer(a)) = v.first() {
                    a == &attr
                } else {
                    false
                }
            }
            _ => false,
        }),
        _ => false,
    }
}

/// Send an envelope, and return the response envelope. Assert that the HTTP endpoint
/// returned a 200 status and the right headers were passed.
pub async fn send(config: &TestConfig, content: CoseSign1) -> CoseSign1 {
    let client = reqwest::Client::new();
    let r = client
        .get(config.url.clone())
        .body(content.to_tagged_vec().unwrap())
        .build()
        .unwrap();

    let result = client.execute(r).await.unwrap();
    let status = result.status();
    assert_eq!(status, StatusCode::from_u16(200).unwrap());

    let bytes = result.bytes().await.unwrap();
    CoseSign1::from_tagged_slice(bytes.to_vec().as_slice()).expect("Bytes were not a CoseSign1")
}

#[derive(Debug, Default)]
pub struct Payload {
    pub version: Option<Value>,
    pub from: Option<Value>,
    pub to: Option<Value>,
    pub endpoint: Option<Value>,
    pub arguments: Option<Value>,
    pub time: Option<Value>,
    pub id: Option<Value>,
    pub nonce: Option<Value>,
    pub attributes: Option<Value>,
}

impl AsCborValue for Payload {
    fn from_cbor_value(_value: Value) -> coset::Result<Self> {
        unreachable!("Unimplemented.")
    }

    fn to_cbor_value(self) -> coset::Result<Value> {
        macro_rules! fields {
            ($map: ident { $($num: literal => $field: ident),* }) => {
                $(
                if let Some(ref v) = self. $field {
                    $map .push((Value::from($num), v.clone()));
                }
                )*
            }
        }

        let mut map: Vec<(Value, Value)> = Vec::new();
        fields!(
            map {
                0 => version,
                1 => from,
                2 => to,
                3 => endpoint,
                4 => arguments,
                5 => time,
                6 => id,
                7 => nonce,
                8 => attributes
            }
        );

        Ok(Value::Map(map))
    }
}

impl coset::CborSerializable for Payload {}

impl coset::TaggedCborSerializable for Payload {
    const TAG: u64 = 10001;
}

fn key_from_pem(pem: String) -> Result<ed25519_dalek::Keypair, String> {
    let doc = pkcs8::PrivateKeyDocument::from_pem(&pem)
    .map_err(|e| {
        format!("{}. Can't read PEM. Did you update the config.json for your leadger with a valid faucet_pk string? ", e)
    })?;
    let decoded = doc.decode();

    let sk = ed25519_dalek::SecretKey::from_bytes(&decoded.private_key[2..])
        .map_err(|e| e.to_string())
        .unwrap();
    let pk: PublicKey = (&sk).into();
    let keypair: ed25519_dalek::Keypair = ed25519_dalek::Keypair {
        secret: sk,
        public: pk,
    };
    Ok(ed25519_dalek::Keypair::from_bytes(&keypair.to_bytes()).unwrap())
}

fn key_from_seed(key_seed: u8) -> ed25519_dalek::Keypair {
    let mut seed = [0u8; 32];
    seed[31] = key_seed;

    let mut prng = rand::rngs::StdRng::from_seed(seed);
    ed25519_dalek::Keypair::generate(&mut prng)
}

/// Returns the key pair, KID and the header for a key seed.
pub fn generate_key(key: KeyType) -> MessageKey {
    let ed25519_key = match key {
        KeyType::PrivateKey(pem) => key_from_pem(pem).unwrap(),
        KeyType::KeySeed(key_seed) => key_from_seed(key_seed),
    };

    let mut pkey = coset::CoseKeyBuilder::new()
        .algorithm(coset::iana::Algorithm::EdDSA)
        .param(
            coset::iana::Ec2KeyParameter::Crv as i64,
            Value::from(coset::iana::EllipticCurve::Ed25519 as u64),
        )
        .param(
            coset::iana::Ec2KeyParameter::X as i64,
            Value::Bytes(ed25519_key.public.as_bytes().to_vec()),
        )
        .add_key_op(coset::iana::KeyOperation::Verify)
        .build();
    pkey.kty = coset::KeyType::Assigned(coset::iana::KeyType::OKP);

    let mut kid = [0u8; 29];
    kid[0] = 1;
    let mut hasher = Sha3_224::default();
    hasher.update(&pkey.clone().to_vec().unwrap());
    kid[1..29].copy_from_slice(&hasher.finalize());

    pkey.key_id = kid.clone().to_vec();

    let keyset = Value::Array(vec![pkey.to_cbor_value().unwrap()]);
    let mut keyset_bytes = Vec::new();
    ciborium::ser::into_writer(&keyset, &mut keyset_bytes).unwrap();

    let protected = coset::HeaderBuilder::new()
        .key_id(kid.clone().to_vec())
        .content_type("application/cbor".to_string())
        .text_value("keyset".to_string(), Value::Bytes(keyset_bytes))
        .build();

    MessageKey {
        ed25519_key: Some(ed25519_key),
        kid: Some(kid.to_vec()),
        protected: Some(protected),
    }
}

/// Create an envelope with a message and an optional key_seed. If the key_seed is
/// specified, the protected headers and signatures will be filled.
pub fn envelope<M: AsRef<[u8]>>(
    message: M,
    (ed25519_key, protected): (Option<ed25519_dalek::Keypair>, Option<Header>),
) -> CoseSign1 {
    let builder = coset::CoseSign1Builder::new().payload(message.as_ref().to_vec());

    let builder = if let Some(protected) = protected {
        builder.protected(protected).create_signature(&[], |bytes| {
            ed25519_key.unwrap().sign(bytes).to_bytes().to_vec()
        })
    } else {
        builder
    };

    builder.build()
}

/// Create message
pub fn create_message<P: AsRef<str>>(endpoint: &str, payload: P, kid: Option<Vec<u8>>) -> Vec<u8> {
    let arg_bytes = cbor_diag::parse_diag(payload.as_ref())
        .expect("Could not parse CBOR.")
        .to_bytes();

    let mut message = Payload {
        version: Some(Value::from(1)),
        endpoint: Some(Value::Text(endpoint.to_string())),
        arguments: Some(Value::Bytes(arg_bytes)),
        time: Some(Value::Tag(
            1,
            Box::new(Value::from(
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
            )),
        )),
        ..Default::default()
    };

    if let Some(kid) = kid {
        message.from = Some(Value::Tag(10000, Box::new(Value::Bytes(kid.to_vec()))));
    }

    message
        .to_tagged_vec()
        .expect("Could not serialize payload")
}

/// Create an anonymous message envelope.
pub fn anonymous_message<P: AsRef<str>>(endpoint: &str, payload: P) -> CoseSign1 {
    message(endpoint, payload, MessageKey::default())
}

/// Create a message envelope.
pub fn message<P: AsRef<str>>(endpoint: &str, payload: P, key: MessageKey) -> CoseSign1 {
    let message = create_message(endpoint, payload, key.kid);
    envelope(message, (key.ed25519_key, key.protected))
}

const PATH_SEPARATOR: char = ':';
const CONFIG_SEPARATOR: char = '=';

pub fn parse_config_paths(config: Option<String>) -> BTreeMap<String, PathBuf> {
    let mut config_paths = BTreeMap::new();

    if let Some(config) = config {
        let split = config.split(PATH_SEPARATOR);
        for s in split {
            let key_path: Vec<&str> = s.split(CONFIG_SEPARATOR).collect();
            let path = PathBuf::from(key_path[1].to_string());
            if path.exists() {
                config_paths.insert(
                    key_path[0].to_string(),
                    PathBuf::from(key_path[1].to_string()),
                );
            } else {
                panic!(
                    "Path for {} does not exist ({})",
                    key_path[0],
                    path.to_string_lossy()
                );
            }
        }
    }

    config_paths
}
