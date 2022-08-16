use many_identity::{cose_helpers::eddsa_cose_key, testing::identity, CoseKeyIdentity};

use crate::IdentityName;

pub fn new_identity(id: &IdentityName) -> Result<CoseKeyIdentity, String> {
    let seed: u32 = id.into();
    let address = identity(seed);
    let cose_key = eddsa_cose_key(address.to_vec(), None);
    CoseKeyIdentity::from_key(cose_key, false)
}
