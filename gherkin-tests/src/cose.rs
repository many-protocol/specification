use many_identity::{testsutils::generate_random_eddsa_identity, CoseKeyIdentity};

pub fn new_identity() -> Result<CoseKeyIdentity, String> {
    Ok(generate_random_eddsa_identity())
}
