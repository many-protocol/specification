use crate::types::Identity;
use coset::iana::{self, Ec2KeyParameter, EnumI64, OkpKeyParameter};
use coset::{Algorithm, CoseKey, KeyOperation, Label};
use signature::{Error, Signature, Signer, Verifier};
use std::collections::BTreeMap;
use std::convert::TryFrom;
use std::fmt::{Debug, Formatter};
use tracing::error;

#[derive(Clone, Eq, PartialEq)]
pub struct CoseKeyIdentitySignature {
    bytes: Vec<u8>,
}

impl Debug for CoseKeyIdentitySignature {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "CoseKeyIdentitySignature(0x{})",
            hex::encode(&self.bytes)
        )
    }
}

impl AsRef<[u8]> for CoseKeyIdentitySignature {
    fn as_ref(&self) -> &[u8] {
        &self.bytes
    }
}

impl Signature for CoseKeyIdentitySignature {
    fn from_bytes(bytes: &[u8]) -> Result<Self, Error> {
        Ok(Self {
            bytes: bytes.to_vec(),
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct CoseKeyIdentity {
    pub identity: Identity,
    pub key: Option<CoseKey>,
    pub hsm: bool,
}

impl Default for CoseKeyIdentity {
    fn default() -> Self {
        Self::anonymous()
    }
}

impl CoseKeyIdentity {
    pub fn anonymous() -> Self {
        Self {
            identity: Identity::anonymous(),
            key: None,
            hsm: false,
        }
    }
}

impl TryFrom<String> for CoseKeyIdentity {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        let identity: Identity = Identity::try_from(value).map_err(|e| e.to_string())?;
        if identity.is_anonymous() {
            Ok(Self {
                identity,
                key: None,
                hsm: false,
            })
        } else {
            Err("Identity must be anonymous".to_string())
        }
    }
}

impl AsRef<Identity> for CoseKeyIdentity {
    fn as_ref(&self) -> &Identity {
        &self.identity
    }
}

impl Verifier<CoseKeyIdentitySignature> for CoseKeyIdentity {
    fn verify(&self, msg: &[u8], signature: &CoseKeyIdentitySignature) -> Result<(), Error> {
        if let Some(cose_key) = self.key.as_ref() {
            match cose_key.alg {
                None => Err(Error::new()),
                Some(Algorithm::Assigned(coset::iana::Algorithm::ES256)) => {
                    let params = BTreeMap::from_iter(cose_key.params.clone().into_iter());
                    let x = params
                        .get(&Label::Int(Ec2KeyParameter::X.to_i64()))
                        .ok_or_else(Error::new)?
                        .as_bytes()
                        .ok_or_else(Error::new)?
                        .as_slice();
                    let y = params
                        .get(&Label::Int(Ec2KeyParameter::Y.to_i64()))
                        .ok_or_else(Error::new)?
                        .as_bytes()
                        .ok_or_else(Error::new)?
                        .as_slice();
                    let points =
                        p256::EncodedPoint::from_affine_coordinates(x.into(), y.into(), false);

                    let verify_key = p256::ecdsa::VerifyingKey::from_encoded_point(&points)?;
                    let signature = p256::ecdsa::Signature::from_der(&signature.bytes)
                        .or_else(|_| p256::ecdsa::Signature::from_bytes(&signature.bytes))?;
                    verify_key.verify(msg, &signature).map_err(|e| {
                        error!("Key verify failed: {}", e);
                        Error::new()
                    })
                }
                Some(Algorithm::Assigned(coset::iana::Algorithm::EdDSA)) => {
                    let params = BTreeMap::from_iter(cose_key.params.clone().into_iter());
                    let x = params
                        .get(&Label::Int(OkpKeyParameter::X.to_i64()))
                        .ok_or_else(Error::new)?;

                    let public_key = ed25519_dalek::PublicKey::from_bytes(
                        x.as_bytes().ok_or_else(Error::new)?.as_slice(),
                    )
                    .map_err(|e| {
                        error!("Public key does not deserialize: {}", e);
                        Error::new()
                    })?;
                    public_key
                        .verify_strict(msg, &ed25519::Signature::from_bytes(&signature.bytes)?)
                        .map_err(|e| {
                            error!("Verification failed (ed25519): {}", e);
                            Error::new()
                        })
                }
                // TODO: Raise a "Algorithm not supported" error
                _ => Err(Error::new()),
            }
        } else {
            Err(Error::new())
        }
    }
}

impl Signer<CoseKeyIdentitySignature> for CoseKeyIdentity {
    fn try_sign(&self, msg: &[u8]) -> Result<CoseKeyIdentitySignature, Error> {
        if let Some(cose_key) = self.key.as_ref() {
            match cose_key.alg {
                None => Err(Error::new()),
                Some(Algorithm::Assigned(coset::iana::Algorithm::EdDSA)) => {
                    if !cose_key
                        .key_ops
                        .contains(&KeyOperation::Assigned(iana::KeyOperation::Sign))
                    {
                        return Err(Error::new());
                    }
                    let params = BTreeMap::from_iter(cose_key.params.clone().into_iter());
                    let x = params
                        .get(&Label::Int(OkpKeyParameter::X.to_i64()))
                        .ok_or_else(Error::new)?
                        .as_bytes()
                        .ok_or_else(Error::new)?
                        .as_slice();
                    let d = params
                        .get(&Label::Int(OkpKeyParameter::D.to_i64()))
                        .ok_or_else(Error::new)?
                        .as_bytes()
                        .ok_or_else(Error::new)?
                        .as_slice();

                    let kp = ed25519_dalek::Keypair::from_bytes(&vec![d, x].concat())
                        .map_err(Error::from_source)?;
                    let s = kp.sign(msg);
                    CoseKeyIdentitySignature::from_bytes(&s.to_bytes())
                }
                // TODO: Raise a "Algorithm not supported" error
                _ => Err(Error::new()),
            }
        } else {
            Err(Error::new())
        }
    }
}
