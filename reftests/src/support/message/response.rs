use crate::support::{protocol::AttributeSet, types::Identity};

use derive_builder::Builder;
use num_derive::{FromPrimitive, ToPrimitive};
use std::time::SystemTime;

#[derive(FromPrimitive, ToPrimitive)]
#[repr(i8)]
pub enum ResponseMessageCborKey {
    ProtocolVersion = 0,
    From,
    To,
    _Endpoint, // Unused in Response.
    Result,
    Timestamp,
    Id,
    _Nonce, // Unused in Response.
    Attributes,
}

/// A MANY message response.
#[derive(Clone, Debug, Builder)]
#[builder(setter(strip_option), default)]
pub struct ResponseMessage {
    pub version: Option<u8>,
    pub from: Identity,
    pub to: Option<Identity>,
    pub data: Result<Vec<u8>, super::ManyError>,

    /// An optional timestamp for this response. If [None] this will be filled
    /// with [SystemTime::now()]
    pub timestamp: Option<SystemTime>,

    pub id: Option<u64>,
    pub attributes: AttributeSet,
}

impl Default for ResponseMessage {
    fn default() -> Self {
        Self {
            version: None,
            from: Identity::anonymous(),
            to: None,
            data: Ok(vec![]),
            timestamp: None,
            id: None,
            attributes: Default::default(),
        }
    }
}
