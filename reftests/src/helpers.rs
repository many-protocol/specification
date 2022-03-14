use crate::TestConfig;
use ciborium::value::{Integer, Value};
use coset::{AsCborValue, CborSerializable, CoseSign1, Header, TaggedCborSerializable};
use ed25519_dalek::Signer;
use rand::SeedableRng;
use reqwest::StatusCode;
use sha3::{Digest, Sha3_224};
use std::time::{SystemTime, UNIX_EPOCH};

pub async fn has_attribute(attr: u32, config: &TestConfig) -> bool {
    let result = send(config, anonymous_message("status", Value::Null)).await;

    let payload = result.payload.expect("No payload from status");
    let status: ciborium::value::Value =
        ciborium::de::from_reader(payload.as_slice()).expect("Invalid payload.");

    let attrs = match status {
        Value::Map(m) => {
            // Find key 4 for attributes.
            m.into_iter()
                .find(|(key, _)| key.as_integer().map_or(false, |i| i == Integer::from(4)))
        }
        _ => panic!("Status was not a map."),
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

impl Payload {
    pub fn to_value(&self) -> Value {
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

        Value::Map(map)
    }

    pub fn to_tagged_value(&self) -> Value {
        Value::Tag(10001, Box::new(self.to_value()))
    }
}

/// Returns the key pair, KID and the header for a key seed.
pub fn generate_key(key_seed: u8) -> (ed25519_dalek::Keypair, Vec<u8>, Header) {
    let mut seed = [0u8; 32];
    seed[31] = key_seed;

    let mut prng = rand::rngs::StdRng::from_seed(seed);
    let ed25519_key = ed25519_dalek::Keypair::generate(&mut prng);

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

    (ed25519_key, kid.to_vec(), protected)
}

/// Create an envelope with a message and an optional key_seed. If the key_seed is
/// specified, the protected headers and signatures will be filled.
pub fn envelope(key_seed: Option<u8>, message: Value) -> CoseSign1 {
    let mut buffer: Vec<u8> = Vec::new();
    ciborium::ser::into_writer(&message, &mut buffer).unwrap();
    let builder = coset::CoseSign1Builder::new().payload(buffer);

    let builder = if let Some(key_seed) = key_seed {
        let (ed25519_key, _kid, protected) = generate_key(key_seed);
        builder
            .protected(protected)
            .create_signature(&[], |bytes| ed25519_key.sign(bytes).to_bytes().to_vec())
    } else {
        builder
    };

    builder.build()
}

/// Create an anonymous message envelope.
pub fn anonymous_message(endpoint: &str, args: Value) -> CoseSign1 {
    let mut arg_bytes = Vec::new();
    ciborium::ser::into_writer(&args, &mut arg_bytes).unwrap();

    let message = Payload {
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
    }
    .to_tagged_value();

    envelope(None, message)
}

/// Create a signed message envelope.
pub fn message(key_seed: u8, endpoint: &str, args: Value) -> CoseSign1 {
    let mut arg_bytes = Vec::new();
    ciborium::ser::into_writer(&args, &mut arg_bytes).unwrap();

    let (_, kid, _) = generate_key(key_seed);
    let message = Payload {
        version: Some(Value::from(1)),
        from: Some(Value::Tag(10000, Box::new(Value::Bytes(kid.to_vec())))),
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
    }
    .to_tagged_value();

    envelope(Some(key_seed), message)
}
