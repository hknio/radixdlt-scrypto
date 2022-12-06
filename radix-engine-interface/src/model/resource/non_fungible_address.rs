use sbor::rust::borrow::ToOwned;
use sbor::rust::fmt;
use sbor::rust::str::FromStr;
use sbor::rust::string::String;
use sbor::rust::vec::Vec;
use sbor::*;

use crate::abi::*;
use crate::address::{Bech32Decoder, Bech32Encoder};
use crate::constants::*;
use crate::crypto::*;
use crate::data::ScryptoCustomTypeId;
use crate::model::*;
use crate::scrypto_type;

/// Identifier for a non-fungible unit.
#[derive(Clone, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub struct NonFungibleAddress {
    resource_address: ResourceAddress,
    non_fungible_id: NonFungibleId,
}

//=======
// error
//=======

/// Represents an error when parsing non-fungible address.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseNonFungibleAddressError {
    InvalidLength(usize),
    InvalidResourceAddress,
    InvalidNonFungibleId,
    InvalidHex(String),
    InvalidPrefix,
    ColonNotFound,
}

#[cfg(not(feature = "alloc"))]
impl std::error::Error for ParseNonFungibleAddressError {}

#[cfg(not(feature = "alloc"))]
impl fmt::Display for ParseNonFungibleAddressError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

//========
// binary
//========

impl TryFrom<&[u8]> for NonFungibleAddress {
    type Error = ParseNonFungibleAddressError;

    fn try_from(slice: &[u8]) -> Result<Self, Self::Error> {
        if slice.len() < 27 {
            return Err(ParseNonFungibleAddressError::InvalidLength(slice.len()));
        }

        let (resource_address_slice, non_fungible_id_slice) = slice.split_at(27);
        let resource_address = ResourceAddress::try_from(resource_address_slice)
            .map_err(|_| ParseNonFungibleAddressError::InvalidResourceAddress)?;
        let non_fungible_id = NonFungibleId::try_from(non_fungible_id_slice)
            .map_err(|_| ParseNonFungibleAddressError::InvalidNonFungibleId)?;
        Ok(NonFungibleAddress {
            resource_address,
            non_fungible_id,
        })
    }
}

impl NonFungibleAddress {
    pub const fn new(resource_address: ResourceAddress, non_fungible_id: NonFungibleId) -> Self {
        Self {
            resource_address,
            non_fungible_id,
        }
    }

    /// Returns the resource address.
    pub fn resource_address(&self) -> ResourceAddress {
        self.resource_address
    }

    /// Returns the non-fungible id.
    pub fn non_fungible_id(&self) -> NonFungibleId {
        self.non_fungible_id.clone()
    }

    pub fn to_vec(&self) -> Vec<u8> {
        let mut vec = self.resource_address.to_vec();
        let mut other_vec = self.non_fungible_id.to_vec();
        vec.append(&mut other_vec);
        vec
    }

    /// Returns canonical representation of this NonFungibleAddress.
    pub fn to_canonical_string(&self, bech32_encoder: &Bech32Encoder) -> String {
        format!(
            "{}:{}",
            bech32_encoder.encode_resource_address_to_string(&self.resource_address),
            self.non_fungible_id.to_simple_string()
        )
    }

    /// Converts canonical representation to NonFungibleAddress.
    pub fn try_from_canonical_string(
        bech32_decoder: &Bech32Decoder,
        id_type: NonFungibleIdType,
        s: &str,
    ) -> Result<Self, ParseNonFungibleAddressError> {
        if let Some(idx) = s.find(":") {
            if idx > 0 && idx < s.len() - 1 {
                if let Ok(raddr) = bech32_decoder.validate_and_decode_resource_address(&s[0..idx]) {
                    if let Ok(nfid) = NonFungibleId::try_from_simple_string(id_type, &s[idx + 1..])
                    {
                        Ok(NonFungibleAddress::new(raddr, nfid))
                    } else {
                        Err(ParseNonFungibleAddressError::InvalidNonFungibleId)
                    }
                } else {
                    Err(ParseNonFungibleAddressError::InvalidResourceAddress)
                }
            } else {
                Err(ParseNonFungibleAddressError::InvalidLength(s.len()))
            }
        } else {
            Err(ParseNonFungibleAddressError::ColonNotFound)
        }
    }
}

scrypto_type!(
    NonFungibleAddress,
    ScryptoCustomTypeId::NonFungibleAddress,
    Type::NonFungibleAddress
);

//======
// text
//======

impl FromStr for NonFungibleAddress {
    type Err = ParseNonFungibleAddressError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let bytes =
            hex::decode(s).map_err(|_| ParseNonFungibleAddressError::InvalidHex(s.to_owned()))?;
        Self::try_from(bytes.as_ref())
    }
}

impl fmt::Display for NonFungibleAddress {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        // Note that if the non-fungible ID is empty, the non-fungible address won't be distinguishable from resource address.
        // TODO: figure out what's best for the users
        write!(f, "{}", hex::encode(&self.to_vec()))
    }
}

impl fmt::Debug for NonFungibleAddress {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "{}", self)
    }
}

pub trait FromPublicKey: Sized {
    fn from_public_key<P: Into<PublicKey> + Clone>(public_key: &P) -> Self;
}

impl FromPublicKey for NonFungibleAddress {
    fn from_public_key<P: Into<PublicKey> + Clone>(public_key: &P) -> Self {
        let public_key: PublicKey = public_key.clone().into();
        match public_key {
            PublicKey::EcdsaSecp256k1(public_key) => NonFungibleAddress::new(
                ECDSA_SECP256K1_TOKEN,
                NonFungibleId::Bytes(hash(public_key.to_vec()).lower_26_bytes().into()),
            ),
            PublicKey::EddsaEd25519(public_key) => NonFungibleAddress::new(
                EDDSA_ED25519_TOKEN,
                NonFungibleId::Bytes(hash(public_key.to_vec()).lower_26_bytes().into()),
            ),
        }
    }
}

//======
// test
//======

#[cfg(test)]
mod tests {
    use super::*;
    use crate::address::Bech32Decoder;
    use sbor::rust::string::ToString;

    #[test]
    pub fn non_fungible_address_from_and_to_string_succeeds() {
        let expected_address = "00a6b27633f925c38a123dcfa488fbed09f3c36e97d055e2d603075c20071630071000000071dba5dd36e30de857049805fd1553cd";

        let resource_address = Bech32Decoder::for_simulator()
            .validate_and_decode_resource_address(
                "resource_sim1qzntya3nlyju8zsj8h86fz8ma5yl8smwjlg9tckkqvrs520k2p",
            )
            .expect("Resource address from str failed.");
        let non_fungible_id = NonFungibleId::Bytes(
            hex::decode("30071000000071dba5dd36e30de857049805fd1553cd")
                .expect("Invalid NonFungibleId hex"),
        );
        let non_fungible_address = NonFungibleAddress::new(resource_address, non_fungible_id);

        let s1 = non_fungible_address.to_vec();
        assert_eq!(hex::encode(s1), expected_address);
    }

    #[test]
    fn non_fungible_address_canonical_conversion() {
        let dec = Bech32Decoder::for_simulator();
        NonFungibleAddress::try_from_canonical_string(
            &dec,
            NonFungibleIdType::U32,
            "resource_sim1qzntya3nlyju8zsj8h86fz8ma5yl8smwjlg9tckkqvrs520k2p:1",
        )
        .unwrap();
        NonFungibleAddress::try_from_canonical_string(
            &dec,
            NonFungibleIdType::U64,
            "resource_sim1qzntya3nlyju8zsj8h86fz8ma5yl8smwjlg9tckkqvrs520k2p:10",
        )
        .unwrap();
        NonFungibleAddress::try_from_canonical_string(
            &dec,
            NonFungibleIdType::UUID,
            "resource_sim1qzntya3nlyju8zsj8h86fz8ma5yl8smwjlg9tckkqvrs520k2p:1234567890",
        )
        .unwrap();
        NonFungibleAddress::try_from_canonical_string(
            &dec,
            NonFungibleIdType::String,
            "resource_sim1qzntya3nlyju8zsj8h86fz8ma5yl8smwjlg9tckkqvrs520k2p:test",
        )
        .unwrap();
        NonFungibleAddress::try_from_canonical_string(
            &dec,
            NonFungibleIdType::Bytes,
            "resource_sim1qzntya3nlyju8zsj8h86fz8ma5yl8smwjlg9tckkqvrs520k2p:010a",
        )
        .unwrap();
    }
}
