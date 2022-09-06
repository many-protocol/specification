use many_identity_dsa::{ecdsa::generate_random_ecdsa_cose_key, CoseKeyIdentity};

pub fn new_identity() -> CoseKeyIdentity {
    let cose_key = generate_random_ecdsa_cose_key();
    CoseKeyIdentity::from_key(&cose_key).expect("Should have generated a random cose key identity")
}
