use many_identity::{cose_helpers::eddsa_cose_key, testing::identity, CoseKeyIdentity};

pub fn new_identity(id: &str) -> Result<CoseKeyIdentity, String> {
    let mut seed = 0;
    for (&b, i) in id.as_bytes().iter().zip(0..) {
        seed += (b as u32) << i;
    }
    let address = identity(seed);
    let cose_key = eddsa_cose_key(address.to_vec(), None);
    CoseKeyIdentity::from_key(cose_key, false)
}
