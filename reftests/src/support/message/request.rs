use derive_builder::Builder;
use minicbor::data::{Tag, Type};
use minicbor::encode::{Error, Write};
use minicbor::{Decode, Decoder, Encode, Encoder};
use num_derive::{FromPrimitive, ToPrimitive};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::support::protocol::AttributeSet;
use crate::support::types::Identity;

#[derive(FromPrimitive, ToPrimitive)]
#[repr(i8)]
pub enum RequestMessageCborKey {
    ProtocolVersion = 0,
    From,
    To,
    Endpoint,
    Argument,
    Timestamp,
    Id,
    Nonce,
    Attributes,
}

#[derive(Clone, Default, Builder)]
#[builder(setter(strip_option), default)]
pub struct RequestMessage {
    pub version: Option<u8>,
    pub from: Option<Identity>,
    pub to: Identity,
    pub method: String,
    pub data: Vec<u8>,

    /// An optional timestamp for this request. If [None] this will be filled
    /// with [SystemTime::now()]
    pub timestamp: Option<SystemTime>,

    pub id: Option<u64>,
    pub nonce: Option<Vec<u8>>,
    pub attributes: AttributeSet,
}

impl std::fmt::Debug for RequestMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let anon = Identity::anonymous();
        let data = hex::encode(&self.data);

        let mut s = f.debug_struct("RequestMessage");
        s.field("version", &self.version)
            .field("from", self.from.as_ref().unwrap_or(&anon))
            .field("to", &self.to)
            .field("method", &self.method)
            .field("data", &data);

        if let Some(timestamp) = &self.timestamp {
            s.field("timestamp", timestamp);
        }
        if let Some(id) = &self.id {
            s.field("id", id);
        }
        if !self.attributes.is_empty() {
            s.field("attributes", &self.attributes);
        }

        s.finish()
    }
}

impl<C> Encode<C> for RequestMessage {
    fn encode<W: Write>(&self, e: &mut Encoder<W>, _: &mut C) -> Result<(), Error<W::Error>> {
        e.tag(Tag::Unassigned(10001))?;
        let l =
            2 + if self.from.is_none() || self.from == Some(Identity::anonymous()) {
                0
            } else {
                1
            } + if self.to.is_anonymous() { 0 } else { 1 }
                + if self.data.is_empty() { 0 } else { 1 }
                + if self.id.is_none() { 0 } else { 1 }
                + if self.nonce.is_none() { 0 } else { 1 }
                + if self.attributes.is_empty() { 0 } else { 1 };
        e.map(l)?;

        // Skip version for this version of the protocol. This message implementation
        // only supports version 1.
        // e.i8(RequestMessageCborKey::ProtocolVersion as i8)?.u8(*v)?;

        // No need to send the anonymous identity.
        if let Some(ref i) = self.from {
            if !i.is_anonymous() {
                e.i8(RequestMessageCborKey::From as i8)?.encode(&i)?;
            }
        }

        if !self.to.is_anonymous() {
            e.i8(RequestMessageCborKey::To as i8)?.encode(&self.to)?;
        }

        e.i8(RequestMessageCborKey::Endpoint as i8)?
            .encode(&self.method)?;

        if !self.data.is_empty() {
            e.i8(RequestMessageCborKey::Argument as i8)?
                .bytes(&self.data)?;
        }

        e.i8(RequestMessageCborKey::Timestamp as i8)?;
        let timestamp = self.timestamp.unwrap_or_else(SystemTime::now);
        e.tag(minicbor::data::Tag::Timestamp)?.u64(
            timestamp
                .duration_since(UNIX_EPOCH)
                .expect("Time flew backward")
                .as_secs(),
        )?;

        if let Some(ref id) = self.id {
            e.i8(RequestMessageCborKey::Id as i8)?.u64(*id)?;
        }

        if let Some(ref nonce) = self.nonce {
            e.i8(RequestMessageCborKey::Nonce as i8)?.bytes(nonce)?;
        }

        if !self.attributes.is_empty() {
            e.i8(RequestMessageCborKey::Attributes as i8)?
                .encode(&self.attributes)?;
        }

        Ok(())
    }
}

impl<'b, C> Decode<'b, C> for RequestMessage {
    fn decode(d: &mut Decoder<'b>, _: &mut C) -> Result<Self, minicbor::decode::Error> {
        if d.tag()? != Tag::Unassigned(10001) {
            return Err(minicbor::decode::Error::message(
                "Invalid tag, expected 10001 for a message.",
            ));
        };

        let mut builder = RequestMessageBuilder::default();

        let mut i = 0;
        let x = d.map()?;
        // Since we don't know if this is a indef map or a regular map, we just loop
        // through items and break when we know the map is done.
        loop {
            if d.datatype()? == Type::Break {
                d.skip()?;
                break;
            }

            match num_traits::FromPrimitive::from_i8(d.i8()?) {
                None => &mut builder,
                Some(RequestMessageCborKey::ProtocolVersion) => {
                    let v = d.u8()?;
                    // Only support version 1.
                    if v != 1 {
                        return Err(minicbor::decode::Error::message("Invalid version."));
                    }
                    builder.version(v)
                }
                Some(RequestMessageCborKey::From) => builder.from(d.decode()?),
                Some(RequestMessageCborKey::To) => builder.to(d.decode()?),
                Some(RequestMessageCborKey::Endpoint) => builder.method(d.decode()?),
                Some(RequestMessageCborKey::Argument) => builder.data(d.bytes()?.to_vec()),
                Some(RequestMessageCborKey::Timestamp) => {
                    // Some logic applies.
                    let t = d.tag()?;
                    if t != minicbor::data::Tag::Timestamp {
                        return Err(minicbor::decode::Error::message("Invalid tag."));
                    }

                    let secs = d.u64()?;
                    let timestamp = std::time::UNIX_EPOCH
                        .checked_add(Duration::from_secs(secs))
                        .ok_or_else(|| {
                            minicbor::decode::Error::message(
                                "duration value can not represent system time",
                            )
                        })?;
                    builder.timestamp(timestamp)
                }
                Some(RequestMessageCborKey::Id) => builder.id(d.u64()?),
                Some(RequestMessageCborKey::Nonce) => builder.nonce(d.bytes()?.to_vec()),
                Some(RequestMessageCborKey::Attributes) => builder.attributes(d.decode()?),
            };

            i += 1;
            if x.map_or(false, |x| i >= x) {
                break;
            }
        }

        builder
            .build()
            .map_err(|_e| minicbor::decode::Error::message("could not build"))
    }
}
