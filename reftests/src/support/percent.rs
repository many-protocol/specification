use minicbor::encode::{Error, Write};
use minicbor::{decode, Decode, Decoder, Encode, Encoder};
use std::ops::Shl;

/// A deterministic (fixed point) percent value that can be multiplied with
/// numbers and rounded down.
#[repr(transparent)]
#[derive(Clone, Copy, Debug)]
#[must_use]
pub struct Percent(pub fixed::types::U32F32);

impl Percent {
    pub fn new(i: u32, fraction: u32) -> Self {
        Self(fixed::types::U32F32::from_bits(
            u64::from(i).shl(32) + u64::from(fraction),
        ))
    }
}

impl<C> Encode<C> for Percent {
    fn encode<W: Write>(&self, e: &mut Encoder<W>, _: &mut C) -> Result<(), Error<W::Error>> {
        e.u64(self.0.to_bits())?;
        Ok(())
    }
}

impl<'b, C> Decode<'b, C> for Percent {
    fn decode(d: &mut Decoder<'b>, _: &mut C) -> Result<Self, decode::Error> {
        Ok(Self(fixed::types::U32F32::from_bits(d.u64()?)))
    }
}
