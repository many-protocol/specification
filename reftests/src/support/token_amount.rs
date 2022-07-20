use minicbor::data::{Tag, Type};
use minicbor::{encode, Decode, Decoder, Encode, Encoder};
use num_bigint::{BigInt, BigUint};
use num_traits::Num;
use serde::de::Unexpected;
use serde::Deserialize;
use std::fmt::{Display, Formatter};
use std::ops::Shr;

use super::percent::Percent;
use super::types::Identity;

/// A Symbol is represented by a non-anonymous identity.
pub type Symbol = Identity;
pub type TokenAmountStorage = BigUint;

#[repr(transparent)]
#[derive(Debug, Default, Hash, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub struct TokenAmount(TokenAmountStorage);

impl TokenAmount {
    pub fn zero() -> Self {
        Self(0u8.into())
    }

    pub fn is_zero(&self) -> bool {
        self.0 == 0u8.into()
    }

    pub fn to_vec(&self) -> Vec<u8> {
        self.0.to_bytes_be()
    }
}

impl std::ops::Mul<Percent> for TokenAmount {
    type Output = TokenAmount;

    fn mul(self, rhs: Percent) -> Self::Output {
        let mut n: BigUint = self.0;
        n = (n * rhs.0.to_bits()).shr(32);
        n.into()
    }
}

macro_rules! op_impl {
    ( $( $t: ty )* ) => {
        $(
            impl std::ops::Add<$t> for TokenAmount {
                type Output = TokenAmount;
                fn add(self, rhs: $t) -> Self::Output { Self(self.0 + Into::<BigUint>::into(rhs)) }
            }

            impl std::ops::Sub<$t> for TokenAmount {
                type Output = TokenAmount;
                fn sub(self, rhs: $t) -> Self::Output { Self(self.0 - Into::<BigUint>::into(rhs)) }
            }

            impl std::ops::Mul<$t> for TokenAmount {
                type Output = TokenAmount;
                fn mul(self, rhs: $t) -> Self::Output { Self(self.0 * Into::<BigUint>::into(rhs)) }
            }

            impl std::ops::AddAssign<$t> for TokenAmount {
                fn add_assign(&mut self, rhs: $t) { self.0 += Into::<BigUint>::into(rhs); }
            }

            impl std::ops::SubAssign<$t> for TokenAmount {
                fn sub_assign(&mut self, rhs: $t) { self.0 -= Into::<BigUint>::into(rhs); }
            }

            impl std::ops::MulAssign<$t> for TokenAmount {
                fn mul_assign(&mut self, rhs: $t) { self.0 *= Into::<BigUint>::into(rhs); }
            }
        )*
    }
}

macro_rules! from_impl {
    ( $( $t: ty )* ) => {
        $(
        impl From<$t> for TokenAmount {
            fn from(v: $t) -> Self {
                Self(v.into())
            }
        }
        )*
    };
}

macro_rules! eq_impl {
    ( $( $t: ty )* ) => {
        $(
            impl PartialEq<$t> for TokenAmount {
                fn eq(&self, other: &$t) -> bool { self.0 == (*other).into() }
            }
        )*
    };
}

op_impl!(u8 u16 u32 u64 u128 TokenAmount BigUint);
from_impl!(u8 u16 u32 u64 u128 BigUint);
eq_impl!(u8 u16 u32 u64 u128);

impl From<TokenAmount> for BigUint {
    fn from(t: TokenAmount) -> BigUint {
        t.0
    }
}

impl PartialEq<BigUint> for TokenAmount {
    fn eq(&self, other: &BigUint) -> bool {
        self.0 == *other
    }
}

impl From<Vec<u8>> for TokenAmount {
    fn from(v: Vec<u8>) -> Self {
        TokenAmount(BigUint::from_bytes_be(v.as_slice()))
    }
}

impl From<TokenAmount> for Vec<u8> {
    fn from(t: TokenAmount) -> Vec<u8> {
        t.0.to_bytes_be()
    }
}

impl TryFrom<num_bigint::BigInt> for TokenAmount {
    type Error = ();

    fn try_from(value: BigInt) -> Result<Self, Self::Error> {
        Ok(Self(value.try_into().map_err(|_| ())?))
    }
}

impl Display for TokenAmount {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.0, f)
    }
}

impl<C> Encode<C> for TokenAmount {
    fn encode<W: encode::Write>(
        &self,
        e: &mut Encoder<W>,
        _: &mut C,
    ) -> Result<(), encode::Error<W::Error>> {
        use num_traits::cast::ToPrimitive;

        // Encode efficiently.
        if let Some(amount) = self.0.to_u64() {
            e.u64(amount)?;
        } else {
            e.tag(Tag::PosBignum)?.bytes(&self.0.to_bytes_be())?;
        }
        Ok(())
    }
}

impl<'b, C> Decode<'b, C> for TokenAmount {
    fn decode(d: &mut Decoder<'b>, _: &mut C) -> Result<Self, minicbor::decode::Error> {
        // Decode either.
        match d.datatype()? {
            Type::Tag => {
                if d.tag()? != Tag::PosBignum {
                    return Err(minicbor::decode::Error::message("Invalid tag."));
                }

                let bytes = d.bytes()?.to_vec();
                Ok(TokenAmount::from(bytes))
            }
            _ => Ok(d.u64()?.into()),
        }
    }
}

// Automatically create variants of the i* deserialization.
macro_rules! decl_token_deserialize {
    ( @signed $( $id: ident => $t: ty ),* $(,)? ) => {
        $(
            fn $id <E>(self, v: $t) -> Result<Self::Value, E> where E: serde::de::Error {
                if v >= 0 {
                    self.visit_u64(v as u64)
                } else {
                    Err(E::invalid_type(Unexpected::Signed(v as i64), &"a positive integer"))
                }
            }
        )*
    };
    ( @unsigned $( $id: ident => $t: ty ),* $(,)? ) => {
        $(
            fn $id <E>(self, v: $t) -> Result<Self::Value, E> where E: serde::de::Error {
                self.visit_u64(v as u64)
            }
        )*
    }
}

impl<'de> Deserialize<'de> for TokenAmount {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::de::Deserializer<'de>,
    {
        struct Visitor;
        impl<'de> serde::de::Visitor<'de> for Visitor {
            type Value = TokenAmount;

            fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
                formatter.write_str("amount in number or string")
            }

            decl_token_deserialize!( @signed
                visit_i8 => i8,
                visit_i16 => i16,
                visit_i32 => i32,
                visit_i64 => i64,
            );
            decl_token_deserialize!( @unsigned
                visit_u8 => u8,
                visit_u16 => u16,
                visit_u32 => u32,
            );

            fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(TokenAmount(v.into()))
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                self.visit_borrowed_str(v)
            }

            fn visit_borrowed_str<E>(self, v: &'de str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                let storage = TokenAmountStorage::from_str_radix(v, 10).map_err(E::custom)?;
                Ok(TokenAmount(storage))
            }

            fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                self.visit_borrowed_str(v.as_str())
            }
        }

        deserializer.deserialize_any(Visitor)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use serde_test::{assert_de_tokens, Token};

    #[test]
    fn serde_token_amount() {
        let token = TokenAmount::from(123u32);
        assert_de_tokens(&token, &[Token::U8(123)]);
        assert_de_tokens(&token, &[Token::U16(123)]);
        assert_de_tokens(&token, &[Token::U32(123)]);
        assert_de_tokens(&token, &[Token::U64(123)]);
        assert_de_tokens(&token, &[Token::I8(123)]);
        assert_de_tokens(&token, &[Token::I16(123)]);
        assert_de_tokens(&token, &[Token::I32(123)]);
        assert_de_tokens(&token, &[Token::I64(123)]);
        assert_de_tokens(&token, &[Token::String("123")]);
    }

    #[test]
    fn serde_token_amount_extra() {
        let token = TokenAmount::from(123456789000u64);
        assert_de_tokens(&token, &[Token::String("123_456_789__000")]);
    }

    #[test]
    fn serde_token_amount_big() {
        // This is 80 bits, larger than U64.
        let token =
            TokenAmount::from(BigUint::from_str_radix("FFFF_FFFF_FFFF_FFFF_FFFF", 16).unwrap());
        assert_de_tokens(
            &token,
            &[Token::String("1_208_925_819_614_629_174_706_175")],
        );
    }
}
