use crate::support::cbor::CborAny;
use minicbor::data::Type;
use minicbor::encode::{Error, Write};
use minicbor::{Decode, Decoder, Encode, Encoder};
use std::cmp::Ordering;
use std::fmt::Debug;

use crate::support::message::ManyError;
use std::collections::BTreeSet;

#[derive(Encode, Decode, Clone, Debug, Default, Ord, PartialOrd, Eq, PartialEq)]
#[cbor(transparent)]
pub struct AttributeSet(#[n(0)] BTreeSet<Attribute>);

impl AttributeSet {
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

pub trait TryFromAttributeSet: Sized {
    fn try_from_set(set: &AttributeSet) -> Result<Self, ManyError>;
}

impl IntoIterator for AttributeSet {
    type Item = Attribute;
    type IntoIter = std::collections::btree_set::IntoIter<Attribute>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl FromIterator<Attribute> for AttributeSet {
    fn from_iter<T: IntoIterator<Item = Attribute>>(iter: T) -> Self {
        Self(BTreeSet::from_iter(iter))
    }
}

pub type AttributeId = u32;

#[derive(Clone, Debug)]
pub struct Attribute {
    pub id: AttributeId,
    pub arguments: Vec<CborAny>,
}

impl Attribute {
    pub const fn id(id: AttributeId) -> Self {
        Self {
            id,
            arguments: vec![],
        }
    }
}

impl PartialEq for Attribute {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id && self.arguments.eq(&other.arguments)
    }
}

impl Eq for Attribute {}

impl PartialOrd for Attribute {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.id.partial_cmp(&other.id)
    }
}

impl Ord for Attribute {
    fn cmp(&self, other: &Self) -> Ordering {
        self.id.cmp(&other.id)
    }
}

impl<C> Encode<C> for Attribute {
    fn encode<W: Write>(&self, e: &mut Encoder<W>, _: &mut C) -> Result<(), Error<W::Error>> {
        if self.arguments.is_empty() {
            e.u32(self.id as u32)?;
        } else {
            e.array(1 + self.arguments.len() as u64)?;
            e.u32(self.id as u32)?;
            for a in &self.arguments {
                e.encode(a)?;
            }
        }

        Ok(())
    }
}

impl<'d, C> Decode<'d, C> for Attribute {
    fn decode(d: &mut Decoder<'d>, _: &mut C) -> Result<Self, minicbor::decode::Error> {
        match d.datatype()? {
            Type::Array | Type::ArrayIndef => {
                let arr = d.array_iter()?.collect::<Result<Vec<CborAny>, _>>()?;
                let (id, arguments) = arr
                    .as_slice()
                    .split_first()
                    .ok_or_else(|| minicbor::decode::Error::message("Invalid empty attribute."))?;

                match id {
                    CborAny::Int(i) if i <= &i64::from(u32::MAX) => Ok(Self {
                        id: *i as u32,
                        arguments: arguments.to_vec(),
                    }),
                    _ => Err(minicbor::decode::Error::message(
                        "Expected an attribute ID.",
                    )),
                }
            }
            _ => Ok(Self::id(d.u32()? as u32)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::support::cbor::tests::arb_cbor;
    use proptest::prelude::*;

    prop_compose! {
        /// Generate an arbitrary Attribute argument vector of size [0, 10[
        fn arb_args()(args in prop::collection::vec(arb_cbor(), 0..10)) -> Vec<CborAny> {
            args
        }
    }

    prop_compose! {
        /// Generate an arbitrary Attribute ID
        fn arb_id()(id in any::<u32>()) -> u32 {
            id
        }
    }

    prop_compose! {
        /// Generate an arbitraty Attribute
        fn arb_attr()(id in arb_id(), arguments in arb_args()) -> Attribute {
            Attribute { id, arguments }
        }
    }

    proptest! {
        #[test]
        fn encode_decode(attr in arb_attr()) {
            let cbor = minicbor::to_vec(attr.clone()).unwrap();
            let attr2: Attribute = minicbor::decode(&cbor).unwrap();
            assert_eq!(attr, attr2);

            #[allow(clippy::unusual_byte_groupings)]
            const HIGH_3_BITS_MASK: u8 = 0b111_00000;
            if attr.arguments.is_empty() {
                // Make sure the CBOR type is an unsigned int
                assert_eq!(cbor[0] & HIGH_3_BITS_MASK, 0b00000000);
            } else {
                // Make sure the CBOR type is an array
                assert_eq!(cbor[0] & HIGH_3_BITS_MASK, 0b10000000);
            }
        }

        #[test]
        fn id(id in arb_id()) {
            let attr = Attribute::id(id);
            assert_eq!(attr.id, id);
            assert_eq!(attr.arguments, vec![]);
        }

        #[test]
        fn ord(id1 in arb_id(), id2 in arb_id()) {
            let mut v = vec![id1, id2];
            v.sort_unstable();
            let (attr1, attr2) = (Attribute::id(v[0]), Attribute::id(v[1]));
            assert_eq!(attr1.cmp(&attr2), Ordering::Less);
            assert_eq!(attr1.partial_cmp(&attr2), Some(Ordering::Less));
        }

        #[test]
        fn debug_fmt(attr in arb_attr()) {
            assert_eq!(format!("Attribute {{ id: {}, arguments: {:?} }}", attr.id, attr.arguments), format!("{:?}", attr));
        }
    }
}
