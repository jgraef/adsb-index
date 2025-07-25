#[cfg(feature = "sqlx")]
mod sqlx;

use std::{
    fmt::{
        Debug,
        Display,
    },
    str::FromStr,
};

#[cfg(feature = "serde")]
use serde_with::{
    DeserializeFromStr,
    SerializeDisplay,
};

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(SerializeDisplay, DeserializeFromStr))]
pub struct IcaoAddress {
    address: u32,
    // todo: store this in the address field
    non_icao: bool,
}

impl IcaoAddress {
    pub const fn from_u32_unchecked(address: u32) -> Self {
        Self {
            address,
            non_icao: false,
        }
    }

    pub fn from_u32(address: u32) -> Option<Self> {
        (address < 0x1000000).then(|| Self::from_u32_unchecked(address))
    }

    pub const fn with_non_icao_flag(self) -> Self {
        Self {
            address: self.address,
            non_icao: true,
        }
    }

    pub fn non_icao(&self) -> bool {
        self.non_icao
    }

    pub fn as_bytes(&self) -> [u8; 3] {
        let b = self.address.to_be_bytes();
        assert!(b[0] == 0);
        [b[1], b[2], b[3]]
    }

    pub fn from_bytes(bytes: [u8; 3]) -> Self {
        let b = [0, bytes[0], bytes[1], bytes[2]];
        Self::from_u32_unchecked(u32::from_be_bytes(b))
    }
}

impl Display for IcaoAddress {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.non_icao {
            write!(f, "~")?;
        }
        write!(f, "{:06x}", self.address)
    }
}

impl Debug for IcaoAddress {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "IcaoAddress({self})")
    }
}

impl FromStr for IcaoAddress {
    type Err = IcaoAddressFromStrError;

    fn from_str(mut s: &str) -> Result<Self, Self::Err> {
        let err = || {
            IcaoAddressFromStrError {
                input: s.to_owned(),
            }
        };
        let mut non_icao = false;
        if s.starts_with('~') {
            non_icao = true;
            s = &s[1..];
        }

        let address = u32::from_str_radix(s, 16).map_err(|_| err())?;
        let mut address = Self::from_u32(address).ok_or_else(err)?;
        address.non_icao = non_icao;
        Ok(address)
    }
}

#[derive(Clone, Debug, thiserror::Error)]
#[error("Invalid ICAO address: {input}")]
pub struct IcaoAddressFromStrError {
    pub input: String,
}

impl From<IcaoAddress> for u32 {
    fn from(value: IcaoAddress) -> Self {
        value.address
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(SerializeDisplay, DeserializeFromStr))]
pub struct Squawk {
    code: u16,
}

impl Squawk {
    /// 0700
    pub const VFR_STANDARD: Self = Self::from_u16_unchecked(0o0700);
    /// 7500
    pub const AIRCRAFT_HIJACKING: Self = Self::from_u16_unchecked(0o7500);
    /// 7600
    pub const RADIO_FAILURE: Self = Self::from_u16_unchecked(0o7600);
    /// 7700
    pub const EMERGENCY: Self = Self::from_u16_unchecked(0o7700);

    pub const fn from_u16_unchecked(code: u16) -> Self {
        Self { code }
    }

    pub fn from_u16(code: u16) -> Option<Self> {
        (code < 010000).then(|| Self::from_u16_unchecked(code))
    }

    /// Decodes the "hex-encoded" squawk
    ///
    /// This encoding is the same as Mode A, but without the ident bit.
    /// All irrelevant bits are ignored.
    pub const fn from_u16_hex(code: u16) -> Self {
        // bit:    f e d c b a 9 8 7 6 5 4 3 2 1
        // squawk: a a a 0 b b b 0 c c c 0 d d d -> aaabbbcccddd

        let code = ((code & 0x7000) >> 3)
            | ((code & 0x0700) >> 2)
            | ((code & 0x0070) >> 1)
            | (code & 0x0007);
        Squawk::from_u16_unchecked(code)
    }

    pub fn as_u16(&self) -> u16 {
        self.code
    }
}

impl Display for Squawk {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:04o}", self.code)
    }
}

impl Debug for Squawk {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Squawk({:04o})", self.code)
    }
}

impl FromStr for Squawk {
    type Err = SquawkFromStrError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let err = || {
            SquawkFromStrError {
                input: s.to_owned(),
            }
        };
        let code = u16::from_str_radix(s, 8).map_err(|_| err())?;
        Self::from_u16(code).ok_or_else(err)
    }
}

#[derive(Clone, Debug, thiserror::Error)]
#[error("Invalid squawk code: {input}")]
pub struct SquawkFromStrError {
    pub input: String,
}

impl From<Squawk> for u16 {
    fn from(value: Squawk) -> Self {
        value.code
    }
}
