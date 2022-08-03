use crate::TestConfig;
use ciborium::value::{Integer, Value};
use coset::{AsCborValue, CborSerializable, CoseKey, CoseSign1, Header, TaggedCborSerializable};
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

#[derive(PartialEq)]
pub enum KeyType {
    KeySeed(u8),
    PrivateKey(String),
    None,
}

pub async fn has_attributes(
    find_attributes: Vec<AttributeId>,
    config: &TestConfig,
) -> Result<(), String> {
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

    for find_attribute in find_attributes.clone().into_iter() {
        let find_attr = Integer::from(find_attribute);
        let result = match attrs {
            Some((_, Value::Array(ref attr_list))) => attr_list.iter().any(|v| match v {
                Value::Integer(i) => i == &find_attr,
                Value::Array(v) => {
                    if let Some(Value::Integer(a)) = v.first() {
                        a == &find_attr
                    } else {
                        false
                    }
                }
                _ => false,
            }),
            other => return Err(format!("Server does not support attributes: {:?}", other)),
        };
        if !result {
            return Err(format!(
                "Server does not support attribute: {:?}",
                find_attribute
            ));
        }
    }
    Ok(())
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

/// Create Keypair from read PEM
fn key_from_pem(pem: String) -> Result<ed25519_dalek::Keypair, String> {
    let doc = pkcs8::PrivateKeyDocument::from_pem(&pem).map_err(|e| e.to_string())?;

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

/// Create Keypair from seed
fn key_from_seed(key_seed: u8) -> ed25519_dalek::Keypair {
    let mut seed = [0u8; 32];
    seed[31] = key_seed;

    let mut prng = rand::rngs::StdRng::from_seed(seed);
    ed25519_dalek::Keypair::generate(&mut prng)
}

/// Create Keypair
pub fn generate_key(key: KeyType) -> ed25519_dalek::Keypair {
    match key {
        KeyType::PrivateKey(pem) => key_from_pem(pem).unwrap(),
        KeyType::KeySeed(key_seed) => key_from_seed(key_seed),
        KeyType::None => key_from_seed(0),
    }
}

/// Create CoseKey used by other helper functions.
fn generate_cose_key(key: &ed25519_dalek::Keypair) -> CoseKey {
    coset::CoseKeyBuilder::new()
        .algorithm(coset::iana::Algorithm::EdDSA)
        .param(
            coset::iana::Ec2KeyParameter::Crv as i64,
            Value::from(coset::iana::EllipticCurve::Ed25519 as u64),
        )
        .param(
            coset::iana::Ec2KeyParameter::X as i64,
            Value::Bytes(key.public.as_bytes().to_vec()),
        )
        .add_key_op(coset::iana::KeyOperation::Verify)
        .build()
}

/// Create kid
pub fn generate_kid(key: &ed25519_dalek::Keypair) -> Vec<u8> {
    let mut pkey = generate_cose_key(key);
    pkey.kty = coset::KeyType::Assigned(coset::iana::KeyType::OKP);

    let mut kid = [0u8; 29];
    kid[0] = 1;
    let mut hasher = Sha3_224::default();
    hasher.update(&pkey.to_vec().unwrap());
    kid[1..29].copy_from_slice(&hasher.finalize());
    kid.to_vec()
}

/// Create protected Header
pub fn generate_header(key: &ed25519_dalek::Keypair, kid: &Vec<u8>) -> Header {
    let mut pkey = generate_cose_key(key);
    pkey.kty = coset::KeyType::Assigned(coset::iana::KeyType::OKP);

    pkey.key_id = kid.to_owned().to_vec();

    let keyset = Value::Array(vec![pkey.to_cbor_value().unwrap()]);
    let mut keyset_bytes = Vec::new();
    ciborium::ser::into_writer(&keyset, &mut keyset_bytes).unwrap();

    coset::HeaderBuilder::new()
        .key_id(kid.clone().to_vec())
        .content_type("application/cbor".to_string())
        .text_value("keyset".to_string(), Value::Bytes(keyset_bytes))
        .build()
}

/// Create content
pub fn generate_content<P: AsRef<str>>(endpoint: &str, payload: P, kid: Vec<u8>) -> Vec<u8> {
    let arg_bytes = cbor_diag::parse_diag(payload.as_ref())
        .expect("Could not parse CBOR.")
        .to_bytes();

    let message = Payload {
        version: Some(Value::from(1)),
        endpoint: Some(Value::Text(endpoint.to_string())),
        arguments: Some(Value::Bytes(arg_bytes)),
        from: Some(Value::Tag(10000, Box::new(Value::Bytes(kid.to_vec())))),
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

    message
        .to_tagged_vec()
        .expect("Could not serialize payload")
}

/// Create an anonymous message envelope.
pub fn anonymous_message<P: AsRef<str>>(endpoint: &str, payload: P) -> CoseSign1 {
    message(endpoint, payload, KeyType::None)
}

/// Create a message envelope.
pub fn message<P: AsRef<str>>(endpoint: &str, payload: P, key: KeyType) -> CoseSign1 {
    let key = generate_key(key);
    let kid: Vec<u8> = generate_kid(&key);
    let header = generate_header(&key, &kid);
    let content = generate_content(endpoint, payload, kid);
    coset::CoseSign1Builder::new()
        .payload(content.to_vec())
        .protected(header)
        .create_signature(&[], |bytes| key.sign(bytes).to_bytes().to_vec())
        .build()
}

/// Create an envelope with a message.
pub fn envelope<M: AsRef<[u8]>>(message: M) -> CoseSign1 {
    coset::CoseSign1Builder::new()
        .payload(message.as_ref().to_vec())
        .build()
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
