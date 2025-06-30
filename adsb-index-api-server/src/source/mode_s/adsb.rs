use std::{
    f64::consts::TAU,
    fmt::{
        Debug,
        Display,
    },
    str::FromStr,
};

use adsb_index_api_types::Squawk;
use bytes::Buf;

use crate::{
    source::mode_s::{
        AltitudeUnit,
        DecodeError,
        cpr::{
            Cpr,
            CprFormat,
        },
        util::{
            decode_frame_aligned_altitude_or_identity_code,
            decode_frame_aligned_encoded_position,
            gillham::decode_gillham_id13,
        },
    },
    util::BufReadBytesExt,
};

/// Reference page 49
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Message {
    NoPosition,
    AircraftIdentification(AircraftIdentification),
    SurfacePosition(SurfacePosition),
    AirbornePosition(AirbornePosition),
    AirborneVelocity(AirborneVelocity),
    TestMessage([u8; 6]),
    SurfaceSystemMessage(SurfaceSystemMessage),
    AircraftStatus(AircraftStatus),
    TargetStateAndStatusInformation(TargetStateAndStatusInformation),
    AircraftOperationalStatus(AircraftOperationalStatus),
    Reserved {
        type_code: u8,
        sub_type: u8,
        data: [u8; 6],
    },
}

impl Message {
    pub fn decode<B: Buf>(buffer: &mut B) -> Result<Self, DecodeError> {
        let byte_0 = buffer.get_u8();
        let type_code = byte_0 >> 3;
        let bits_6_to_8 = byte_0 & 0b111; // subtype code for some type codes

        tracing::debug!(?type_code, sub_type = ?bits_6_to_8, "decoding adsb-b message");

        let reserved = |buffer: &mut B| {
            tracing::debug!(?type_code, sub_type = ?bits_6_to_8, "reserved adsb-b message");
            Self::Reserved {
                type_code,
                sub_type: bits_6_to_8,
                data: buffer.get_bytes(),
            }
        };

        let message = match type_code {
            0 => {
                //Self::NoPosition
                todo!("no position");
            }
            1..=4 => {
                Self::AircraftIdentification(AircraftIdentification::decode(
                    buffer,
                    type_code,
                    bits_6_to_8,
                )?)
            }
            5..=8 => Self::SurfacePosition(SurfacePosition::decode(buffer, type_code, bits_6_to_8)),
            9..=18 | 20..=22 => {
                Self::AirbornePosition(AirbornePosition::decode(buffer, type_code, bits_6_to_8))
            }
            19 => {
                match bits_6_to_8 {
                    1..=4 => Self::AirborneVelocity(AirborneVelocity::decode(buffer, bits_6_to_8)),
                    _ => reserved(buffer),
                }
            }
            23 => {
                match bits_6_to_8 {
                    0 => Self::TestMessage(buffer.get_bytes()),
                    _ => reserved(buffer),
                }
            }
            24 => Self::SurfaceSystemMessage(SurfaceSystemMessage::decode(buffer, bits_6_to_8)),
            27 => todo!("reserved for trajectory change message"),
            28 => Self::AircraftStatus(AircraftStatus::decode(buffer, bits_6_to_8)),
            29 => {
                // rare 2-bit sub type
                let sub_type = bits_6_to_8 >> 1;
                match sub_type {
                    1 => {
                        Self::TargetStateAndStatusInformation(
                            TargetStateAndStatusInformation::decode(buffer, bits_6_to_8 & 1 != 0),
                        )
                    }
                    _ => reserved(buffer),
                }
            }
            31 => {
                Self::AircraftOperationalStatus(AircraftOperationalStatus::decode(
                    buffer,
                    bits_6_to_8,
                ))
            }
            _ => reserved(buffer),
        };

        Ok(message)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AircraftIdentification {
    pub wake_vortex_category: WakeVortexCategory,
    pub callsign: Callsign,
}

impl AircraftIdentification {
    pub fn decode<B: Buf>(
        buffer: &mut B,
        type_code: u8,
        bits_6_to_8: u8,
    ) -> Result<Self, DecodeError> {
        Ok(Self {
            wake_vortex_category: WakeVortexCategory::from_type_code_and_category_unchecked(
                type_code,
                bits_6_to_8,
            ),
            callsign: Callsign::from_bytes(buffer.get_bytes())?,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SurfacePosition {
    pub ground_speed: Movement,
    pub ground_track: Option<GroundTrack>,
    pub time: bool,
    pub cpr_format: CprFormat,
    pub cpr_position: Cpr,
}

impl SurfacePosition {
    pub fn decode<B: Buf>(buffer: &mut B, _type_code: u8, bits_6_to_8: u8) -> Self {
        let bytes: [u8; 6] = buffer.get_bytes();
        let (cpr_format, cpr_position) = decode_frame_aligned_encoded_position(&bytes[1..]);
        Self {
            ground_speed: Movement((bits_6_to_8 << 4) | (bytes[0] >> 4)),
            ground_track: if bytes[0] & 0b00001000 == 0 {
                None
            }
            else {
                Some(GroundTrack((bytes[0] << 5) | (bytes[1] >> 4)))
            },
            time: bytes[1] & 0b00001000 != 0,
            cpr_format,
            cpr_position,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AirbornePosition {
    pub altitude_type: AltitudeType,
    pub surveillance_status: SurveillanceStatus,
    pub single_antenna_flag: bool,
    pub encoded_altitude: AltitudeCode,
    pub time: bool,
    pub cpr_format: CprFormat,
    pub cpr_position: Cpr,
}

impl AirbornePosition {
    pub fn decode<B: Buf>(buffer: &mut B, type_code: u8, bits_6_to_8: u8) -> Self {
        let bytes: [u8; 6] = buffer.get_bytes();
        let (cpr_format, cpr_position) = decode_frame_aligned_encoded_position(&bytes[1..]);
        Self {
            //       -1        0        1        2        3        4        5
            // tttttssS aaaaaaaa aaaaTFll llllllll lllllllL LLLLLLLL LLLLLLLL
            altitude_type: AltitudeType::from_type_code(type_code),
            surveillance_status: SurveillanceStatus(bits_6_to_8 >> 1),
            single_antenna_flag: bits_6_to_8 & 0b1 == 1,
            encoded_altitude: AltitudeCode(u16::from(bytes[0] << 4) | u16::from(bytes[1] >> 4)),
            time: bytes[2] & 0b00001000 != 0,
            cpr_format,
            cpr_position,
        }
    }

    pub fn altitude(&self) -> Option<DecodedAltitude> {
        self.encoded_altitude.decode(self.altitude_type)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AirborneVelocity {
    pub supersonic: bool,
    pub intent_change_flag: bool,
    /// deprecated
    pub ifr_capability_flag: bool,
    pub navigation_uncertainty_category: NavigationUncertaintyCategory,
    pub velocity_type: VelocityType,
    pub vertical_rate: VerticalRate,
    /// deprecated
    pub turn_indicator: TurnIndicator,
    pub altitude_difference: AltitudeDifference,
}

impl AirborneVelocity {
    pub fn decode<B: Buf>(buffer: &mut B, bits_6_to_8: u8) -> Self {
        let sub_type = bits_6_to_8;
        let supersonic = sub_type == 3 || sub_type == 4;
        let bytes: [u8; 6] = buffer.get_bytes();

        // byte               0        1        2        3        4        5
        // bit         01234567 01234567 01234567 01234567 01234567 01234567
        // field       abcccdee eeeeeeee fggggggg ggghijjj jjjjjjkk lmmmmmmm

        // a
        let intent_change_flag = bytes[0] & 0b10000000 != 0;
        // b
        let ifr_capability_flag = bytes[0] & 0b01000000 != 0;
        // c
        let navigation_uncertainty_category =
            NavigationUncertaintyCategory((bytes[0] & 0b00111000) >> 3);

        // decode d, e, f, g now, because we need them for both subtypes
        let d = bytes[0] & 0b00000100 != 0;
        let e = (u16::from(bytes[0] & 0b11000000) << 8) | u16::from(bytes[1]);
        let f = bytes[2] & 0b1000000 != 0;
        let g = (u16::from(bytes[2] & 0b01111111) << 3) | u16::from(bytes[3] >> 5);
        let velocity = |v| (v != 0).then(|| Velocity(v));

        // sub-type specific
        let velocity_type = match sub_type {
            1 | 2 => {
                // ground speed

                // d
                let direction_east_west = if d {
                    DirectionEastWest::EastToWest
                }
                else {
                    DirectionEastWest::WestToEast
                };
                // e
                let velocity_east_west = velocity(e);

                // f
                let direction_north_south = if f {
                    DirectionNorthSouth::NorthToSouth
                }
                else {
                    DirectionNorthSouth::SouthToNorth
                };
                // g
                let velocity_north_south = velocity(g);

                VelocityType::GroundSpeed(GroundSpeed {
                    direction_east_west,
                    velocity_east_west,
                    direction_north_south,
                    velocity_north_south,
                })
            }
            3 | 4 => {
                // airspeed

                let magnetic_heading = d.then(|| MagneticHeading(e));
                let airspeed_type = if f {
                    AirspeedType::True
                }
                else {
                    AirspeedType::Indicated
                };
                let airspeed_value = velocity(g);

                VelocityType::Airspeed(Airspeed {
                    magnetic_heading,
                    airspeed_type,
                    airspeed_value,
                })
            }
            _ => panic!("Invalid sub type for AirborneVelocity: {}", sub_type),
        };

        // h
        let vertical_rate_source = if bytes[3] & 0b00010000 == 0 {
            VerticalRateSource::Gnss
        }
        else {
            VerticalRateSource::Barometric
        };

        // i
        let vertical_rate_sign = if bytes[3] & 0b00001000 == 0 {
            VerticalRateSign::Up
        }
        else {
            VerticalRateSign::Down
        };

        // j
        let j = (u16::from(bytes[3]) << 6) | u16::from(bytes[4] >> 2);
        let vertical_rate_value = (j != 0).then(|| VerticalRateValue(j));

        let vertical_rate = VerticalRate {
            source: vertical_rate_source,
            sign: vertical_rate_sign,
            value: vertical_rate_value,
        };

        // k
        let turn_indicator = TurnIndicator(bytes[4] & 0b00000011);

        // l
        let altitude_difference_sign = if bytes[5] & 0b10000000 == 0 {
            AltitudeDifferenceSign::GnssAboveBarometric
        }
        else {
            AltitudeDifferenceSign::GnssBelowBarometric
        };

        // m
        let m = bytes[5] & 0b01111111;
        let altitude_difference_value = (m != 0).then(|| AltitudeDifferenceValue(m));

        let altitude_difference = AltitudeDifference {
            sign: altitude_difference_sign,
            value: altitude_difference_value,
        };

        Self {
            supersonic,
            intent_change_flag,
            ifr_capability_flag,
            navigation_uncertainty_category,
            velocity_type,
            vertical_rate,
            turn_indicator,
            altitude_difference,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AircraftStatus {
    EmergencyPriorityStatusAndModeACode(EmergencyPriorityStatusAndModeACode),
}

impl AircraftStatus {
    pub fn decode<B: Buf>(buffer: &mut B, bits_6_to_8: u8) -> Self {
        let sub_type = bits_6_to_8;

        match sub_type {
            1 => {
                // tttttsss eeeiiiii iiiiiiii
                let bytes: [u8; 2] = buffer.get_bytes();
                Self::EmergencyPriorityStatusAndModeACode(EmergencyPriorityStatusAndModeACode {
                    emergency_priority_status: EmergencyPriorityStatus(bytes[0] >> 5),
                    mode_a_code: {
                        // todo: should this include the ident bit? or should it always be zero?
                        // (page 139). i think it should be the latter.
                        Squawk::from_u16_unchecked(decode_gillham_id13(
                            decode_frame_aligned_altitude_or_identity_code(&bytes[..]),
                        ))
                    },
                    reserved: buffer.get_u32(),
                })
            }
            2 => {
                todo!("1090ES TCAS Resolution Advisory (RA) Broadcast Message (Subtype=2)")
            }
            _ => todo!(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EmergencyPriorityStatusAndModeACode {
    pub emergency_priority_status: EmergencyPriorityStatus,
    pub mode_a_code: Squawk,
    pub reserved: u32,
}

impl EmergencyPriorityStatusAndModeACode {
    pub fn from_squawk(squawk: Squawk) -> Self {
        Self {
            emergency_priority_status: EmergencyPriorityStatus::from_squawk(squawk)
                .unwrap_or_default(),
            mode_a_code: squawk,
            reserved: 0,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EmergencyPriorityStatus(u8);

impl EmergencyPriorityStatus {
    pub const NO_EMERGENCY: Self = Self(0);
    pub const GENERAL_EMERGENCY: Self = Self(1);
    pub const LIFEGUARD_MEDICAL_EMERGENCY: Self = Self(2);
    pub const MINIMAL_FUEL: Self = Self(3);
    pub const NO_COMMUNICATIONS: Self = Self(4);
    pub const UNLAWFUL_INTERFERENCE: Self = Self(5);
    pub const DOWNED_AIRCRAFT: Self = Self(6);

    pub const fn from_u8_unchecked(byte: u8) -> Self {
        Self(byte)
    }

    pub const fn from_u8(byte: u8) -> Option<Self> {
        if byte & 0b11111000 == 0 {
            Some(Self(byte))
        }
        else {
            None
        }
    }

    pub fn as_u8(&self) -> u8 {
        self.0
    }

    pub fn is_emergency(&self) -> bool {
        *self != Self::NO_EMERGENCY
    }

    /// Returns the emergency priority status that shall be set for a given Mode
    /// A code (squawk).
    ///
    /// See 2.2.3.2.7.8.1.1 (page 138)
    pub fn from_squawk(squawk: Squawk) -> Option<Self> {
        match squawk {
            Squawk::AIRCRAFT_HIJACKING => Some(Self::UNLAWFUL_INTERFERENCE),
            Squawk::RADIO_FAILURE => Some(Self::NO_COMMUNICATIONS),
            Squawk::EMERGENCY => Some(Self::GENERAL_EMERGENCY),
            _ => None,
        }
    }
}

impl Default for EmergencyPriorityStatus {
    fn default() -> Self {
        Self::NO_EMERGENCY
    }
}

impl Debug for EmergencyPriorityStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            Self::NO_EMERGENCY => write!(f, "EmergencyPriorityStatus::NO_EMERGENCY"),
            Self::GENERAL_EMERGENCY => write!(f, "EmergencyPriorityStatus::GENERAL_EMERGENCY"),
            Self::LIFEGUARD_MEDICAL_EMERGENCY => {
                write!(f, "EmergencyPriorityStatus::LIFEGUARD_MEDICAL_EMERGENCY")
            }
            Self::MINIMAL_FUEL => write!(f, "EmergencyPriorityStatus::MINIMAL_FUEL"),
            Self::NO_COMMUNICATIONS => write!(f, "EmergencyPriorityStatus::NO_COMMUNICATIONS"),
            Self::UNLAWFUL_INTERFERENCE => {
                write!(f, "EmergencyPriorityStatus::UNLAWFUL_INTERFERENCE")
            }
            Self::DOWNED_AIRCRAFT => write!(f, "EmergencyPriorityStatus::DOWNED_AIRCRAFT"),
            _ => write!(f, "EmergencyPriorityStatus({})", self.0),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TargetStateAndStatusInformation {
    pub sil_supplement: SilSupplement,
    pub selected_altitude_type: SelectedAltitudeType,
    //pub navigation_accuracy_category_position: NavigationAccuracyCategoryPosition,
    /// Feel free to open a pull request :3
    pub todo: (),
}

impl TargetStateAndStatusInformation {
    pub fn decode<B: Buf>(buffer: &mut B, bit_8: bool) -> Self {
        // page 106
        let sil_supplement = if bit_8 {
            SilSupplement::PerSample
        }
        else {
            SilSupplement::PerHour
        };
        let bytes: [u8; 6] = buffer.get_bytes();
        let selected_altitude_type = if bytes[0] & 0b1000000 == 0 {
            SelectedAltitudeType::Fms
        }
        else {
            SelectedAltitudeType::McpFcu
        };

        // todo
        Self {
            sil_supplement,
            selected_altitude_type,
            todo: (),
        }
    }
}

/// Probability of exceeding NIC radius of containment is based on
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SilSupplement {
    PerHour,
    PerSample,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SelectedAltitudeType {
    McpFcu,
    Fms,
}

/// Aircraft Operational Status ADS-B Message
///
/// type=31, page 116
///
/// todo: there's a ADS-B version in here, other good info too :)
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AircraftOperationalStatus {
    Airborne {
        /// Feel free to open a PR
        todo: (),
        // todo airborne participants
    },
    Surface {
        /// Feel free to open a PR
        todo: (),
        // todo surface participants
    },
    Reserved {
        sub_type: u8,
        data: [u8; 6],
    },
}

impl AircraftOperationalStatus {
    pub fn decode<B: Buf>(buffer: &mut B, bits_6_to_8: u8) -> Self {
        let sub_type = bits_6_to_8;

        match sub_type {
            0 => {
                Self::Airborne { todo: () }
                //todo!("airborne")
            }
            1 => {
                Self::Surface { todo: () }
                //todo!("surface")
            }
            _ => {
                Self::Reserved {
                    sub_type,
                    data: buffer.get_bytes(),
                }
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SurfaceSystemMessage {
    Reserved { sub_type: u8, data: [u8; 6] },
    MultilaterationSystemStatus([u8; 6]),
}

impl SurfaceSystemMessage {
    pub fn decode<B: Buf>(buffer: &mut B, bits_6_to_8: u8) -> Self {
        let sub_type = bits_6_to_8;

        match sub_type {
            1 => Self::MultilaterationSystemStatus(buffer.get_bytes()),
            _ => {
                Self::Reserved {
                    sub_type,
                    data: buffer.get_bytes(),
                }
            }
        }
    }
}

/// <https://mode-s.org/1090mhz/content/ads-b/2-identification.html>
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum WakeVortexCategory {
    Reserved { type_code: u8, category: u8 },
    NoCategoryInformation { type_code: u8 },
    SurfaceEmergencyVehicle,
    SurfaceServiceVehicle,
    GroundObstruction { category: u8 },
    GliderSailplane,
    LighterThanAir,
    ParachutistSkydiver,
    UltralightHangGliderParaGlider,
    UnmannedAerialVehicle,
    SpaceTransatmospherricVehicle,
    Light,
    Medium1,
    Medium2,
    HighVortexAirrcraft,
    Heavy,
    HighPerformance,
    Rotorcraft,
}

impl WakeVortexCategory {
    pub const fn from_type_code_and_category_unchecked(type_code: u8, category: u8) -> Self {
        match (type_code, category) {
            (_, 0) => Self::NoCategoryInformation { type_code },
            (2, 1) => Self::SurfaceEmergencyVehicle,
            (2, 3) => Self::SurfaceServiceVehicle,
            (2, 4..=7) => Self::GroundObstruction { category },
            (3, 1) => Self::GliderSailplane,
            (3, 2) => Self::LighterThanAir,
            (3, 3) => Self::ParachutistSkydiver,
            (3, 4) => Self::UltralightHangGliderParaGlider,
            (3, 6) => Self::UnmannedAerialVehicle,
            (3, 7) => Self::SpaceTransatmospherricVehicle,
            (4, 1) => Self::Light,
            (4, 2) => Self::Medium1,
            (4, 3) => Self::Medium2,
            (4, 4) => Self::HighVortexAirrcraft,
            (4, 5) => Self::Heavy,
            (4, 6) => Self::HighPerformance,
            (4, 7) => Self::Rotorcraft,

            _ => {
                Self::Reserved {
                    type_code,
                    category,
                }
            }
        }
    }

    pub const fn from_type_code_and_category(type_code: u8, category: u8) -> Option<Self> {
        if type_code & 0b11100000 == 0 && category & 0b00000111 == 0 {
            Some(Self::from_type_code_and_category_unchecked(
                type_code, category,
            ))
        }
        else {
            None
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Callsign {
    // note: we verified that this is valid ascii, and thus we can also create utf-8 strs from it.
    characters: [u8; Self::LENGTH],
}

impl Callsign {
    pub const LENGTH: usize = 8;

    pub fn from_bytes(bytes: [u8; 6]) -> Result<Self, DecodeError> {
        // byte 0        1        2        3        4        5
        // bit  01234567 01234567 01234567 01234567 01234567 01234567
        // char 00000011 11112222 22333333 44444455 55556666 66777777

        // expand into 8 bits per character
        let mut expanded = [
            bytes[0] >> 2,
            ((bytes[0] & 0b11) << 4) | (bytes[1] >> 4),
            ((bytes[1] & 0b1111) << 2) | (bytes[2] >> 6),
            (bytes[2] & 0b111111),
            bytes[3] >> 2,
            ((bytes[3] & 0b11) << 4) | (bytes[4] >> 4),
            ((bytes[4] & 0b1111) << 2) | (bytes[5] >> 6),
            (bytes[5] & 0b111111),
        ];

        // resolve to ascii character
        for byte in &mut expanded {
            let resolved = CALLSIGN_ENCODING[*byte as usize];

            if resolved == b'#' {
                return Err(DecodeError::InvalidCallsign {
                    encoding: bytes,
                    invalid_byte: resolved,
                });
            }

            *byte = resolved;
        }

        Ok(Self {
            characters: expanded,
        })
    }

    pub fn as_str(&self) -> &str {
        // we check this, so we might use the unsafe variant here
        std::str::from_utf8(&self.characters).expect("bug: invalid utf-8 in callsign")
    }
}

impl Debug for Callsign {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Callsign").field(&self.as_str()).finish()
    }
}

impl Display for Callsign {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl FromStr for Callsign {
    type Err = InvalidCallsign;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let n = s.len();
        if n > Self::LENGTH {
            return Err(InvalidCallsign::InvalidLength(n));
        }

        let mut characters = [0u8; Self::LENGTH];
        for (i, c) in s.chars().enumerate() {
            if !valid_callsign_char(c) {
                return Err(InvalidCallsign::InvalidChar {
                    position: i,
                    character: c,
                });
            }
            characters[i] = c.try_into().unwrap();
        }

        Ok(Self { characters })
    }
}

impl AsRef<[u8]> for Callsign {
    fn as_ref(&self) -> &[u8] {
        &self.characters[..]
    }
}

impl AsRef<[u8; Self::LENGTH]> for Callsign {
    fn as_ref(&self) -> &[u8; Self::LENGTH] {
        &self.characters
    }
}

#[derive(Clone, Debug, thiserror::Error)]
pub enum InvalidCallsign {
    #[error("Invalid character in callsign: '{character}' at position {position}")]
    InvalidChar { position: usize, character: char },
    #[error("Invalid length for callsign: {0}")]
    InvalidLength(usize),
}

/// <https://mode-s.org/1090mhz/content/ads-b/2-identification.html>
pub const CALLSIGN_ENCODING: &'static [u8] =
    b"#ABCDEFGHIJKLMNOPQRSTUVWXYZ##### ###############0123456789######";

pub fn valid_callsign_char(c: char) -> bool {
    c.is_ascii_uppercase() || c.is_ascii_digit() || c == ' '
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Movement(u8);

impl Movement {
    pub const fn from_u8_unchecked(byte: u8) -> Self {
        Self(byte)
    }

    pub const fn from_u8(byte: u8) -> Option<Self> {
        if byte & 0b10000000 == 0 {
            Some(Self(byte))
        }
        else {
            None
        }
    }

    pub fn as_u8(&self) -> u8 {
        self.0
    }

    /// Decode movement in 1/8th knots
    pub fn decode_as_1_8th_kt(&self) -> Option<u32> {
        let q = GroundSpeedQuantization::from_encoded_value(self.0);
        match q {
            GroundSpeedQuantization::NotAvailable => None,
            GroundSpeedQuantization::Stopped => Some(0),
            GroundSpeedQuantization::Quantized {
                encoded_base,
                decoded_base,
                decoded_step,
            } => Some(u32::from(self.0 - *encoded_base) * *decoded_step + *decoded_base),
            GroundSpeedQuantization::Exceeding175Kt => Some(1400),
            GroundSpeedQuantization::Reserved => None,
        }
    }

    /// Decode movement in knots
    pub fn decode(&self) -> Option<f64> {
        self.decode_as_1_8th_kt().map(|speed| speed as f64 * 0.125)
    }
}

impl Debug for Movement {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(kt) = self.decode() {
            write!(f, "GroundSpeed({} kt)", kt)
        }
        else {
            write!(f, "GroundSpeed(None)")
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GroundTrack(u8);

impl GroundTrack {
    pub const fn from_u8_unchecked(byte: u8) -> Self {
        Self(byte)
    }

    pub const fn from_u8(byte: u8) -> Option<Self> {
        if byte & 0b10000000 == 0 {
            Some(Self(byte))
        }
        else {
            None
        }
    }

    pub fn as_u8(&self) -> u8 {
        self.0
    }

    pub fn as_radians(&self) -> f64 {
        std::f64::consts::TAU * (self.0 as f64) / 128.0
    }

    pub fn as_degrees(&self) -> f64 {
        360.0 * (self.0 as f64) / 128.0
    }
}

impl Debug for GroundTrack {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "GroundTrack({:.1}°)", self.as_degrees())
    }
}

// todo: needs a neater interface to make it public
#[derive(Clone, Copy, Debug)]
enum GroundSpeedQuantization {
    NotAvailable,
    Stopped,
    Quantized {
        encoded_base: u8,
        decoded_base: u32, // in 1/8 kt
        decoded_step: u32, // in 1/8 kt
    },
    Exceeding175Kt,
    Reserved,
}

impl GroundSpeedQuantization {
    pub fn from_encoded_value(encoded: u8) -> &'static Self {
        match encoded {
            0 => &Self::NotAvailable,
            1 => &Self::Stopped,
            2..=8 => {
                &Self::Quantized {
                    encoded_base: 2,
                    decoded_base: 1,
                    decoded_step: 1,
                }
            }
            9..=12 => {
                &Self::Quantized {
                    encoded_base: 9,
                    decoded_base: 8,
                    decoded_step: 2,
                }
            }
            13..=38 => {
                &Self::Quantized {
                    encoded_base: 13,
                    decoded_base: 16,
                    decoded_step: 4,
                }
            }
            39..=93 => {
                &Self::Quantized {
                    encoded_base: 39,
                    decoded_base: 120,
                    decoded_step: 8,
                }
            }
            94..=108 => {
                &Self::Quantized {
                    encoded_base: 94,
                    decoded_base: 560,
                    decoded_step: 16,
                }
            }
            109..=123 => {
                &Self::Quantized {
                    encoded_base: 109,
                    decoded_base: 800,
                    decoded_step: 40,
                }
            }
            124 => &Self::Exceeding175Kt,
            _ => &Self::Reserved,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AltitudeType {
    Barometric,
    Gnss,
}

impl AltitudeType {
    pub fn from_type_code(type_code: u8) -> Self {
        match type_code {
            9..=18 => Self::Barometric,
            20..=22 => Self::Gnss,
            _ => panic!("invalid type code: {}", type_code),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SurveillanceStatus(u8);

impl SurveillanceStatus {
    pub const NO_CONDITION: Self = Self(0);
    pub const PERMANENT_ALERT: Self = Self(1);
    pub const TEMPORARY_ALERT: Self = Self(2);
    pub const SPI_CONDITION: Self = Self(3);

    pub const fn from_u8_unchecked(byte: u8) -> Self {
        Self(byte)
    }

    pub const fn from_u8(byte: u8) -> Option<Self> {
        if byte & 0b11111100 == 0 {
            Some(Self(byte))
        }
        else {
            None
        }
    }

    pub fn as_u8(&self) -> u8 {
        self.0
    }
}

impl Debug for SurveillanceStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            Self::NO_CONDITION => write!(f, "SurveillanceStatus::NO_CONDITION"),
            Self::PERMANENT_ALERT => write!(f, "SurveillanceStatus::PERMANENT_ALERT"),
            Self::TEMPORARY_ALERT => write!(f, "SurveillanceStatus::TEMPORARY_ALERT"),
            Self::SPI_CONDITION => write!(f, "SurveillanceStatus::SPI_CONDITION"),
            _ => panic!("invalid SurveillanceStatus bitpattern: {}", self.0),
        }
    }
}

/// 12-bit altitude code
///
/// page 59
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AltitudeCode(u16);

impl AltitudeCode {
    pub const fn from_u16_unchecked(word: u16) -> Self {
        Self(word)
    }

    pub const fn from_u16(word: u16) -> Option<Self> {
        if word & 0b1111000000000000 == 0 {
            Some(Self(word))
        }
        else {
            None
        }
    }

    pub fn as_u16(&self) -> u16 {
        self.0
    }

    pub fn decode(&self, altitude_type: AltitudeType) -> Option<DecodedAltitude> {
        // note: 11 bits altitude with 25 feet resolution and -1000 feet offset gives a
        // max value of 50175, so we need a i32 for the decoded altitude

        if self.0 == 0 {
            None
        }
        else {
            let q_bit = self.0 & 0b000000010000 != 0;

            if q_bit {
                Some(DecodedAltitude {
                    altitude_type,
                    altitude: i32::from((self.0 >> 5) | (self.0 & 0b1111)) * 25 - 1000,
                })
            }
            else {
                todo!();
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DecodedAltitude {
    pub altitude_type: AltitudeType,
    pub altitude: i32,
}

impl DecodedAltitude {
    pub fn unit(&self) -> AltitudeUnit {
        match self.altitude_type {
            AltitudeType::Barometric => AltitudeUnit::Feet,
            AltitudeType::Gnss => AltitudeUnit::Meter,
        }
    }

    pub fn as_meter(&self) -> f64 {
        let a = self.altitude as f64;
        match self.altitude_type {
            AltitudeType::Barometric => 0.3048 * a,
            AltitudeType::Gnss => a,
        }
    }

    pub fn as_ft(&self) -> f64 {
        let a = self.altitude as f64;
        match self.altitude_type {
            AltitudeType::Barometric => a,
            AltitudeType::Gnss => 3.28084 * a,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VelocityType {
    GroundSpeed(GroundSpeed),
    Airspeed(Airspeed),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GroundSpeed {
    pub direction_east_west: DirectionEastWest,
    pub velocity_east_west: Option<Velocity>,
    pub direction_north_south: DirectionNorthSouth,
    pub velocity_north_south: Option<Velocity>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DirectionNorthSouth {
    SouthToNorth,
    NorthToSouth,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DirectionEastWest {
    WestToEast,
    EastToWest,
}

/// A 10-bit velocity value.
///
/// This is used for east-west and north-south ground speed in [`GroundSpeed`]
/// and for the airspeed in [`Airspeed`]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Velocity(u16);

impl Velocity {
    pub const fn from_u16_unchecked(word: u16) -> Self {
        Self(word)
    }

    pub const fn from_u16(word: u16) -> Option<Self> {
        if word & 0b1111110000000000 == 0 && word != 0 {
            Some(Self(word))
        }
        else {
            None
        }
    }

    pub fn as_u16(&self) -> u16 {
        self.0
    }

    pub fn as_knots(&self, supersonic: bool) -> u16 {
        let v = self.0 - 1;
        let v = if supersonic { v * 4 } else { v };
        v
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Airspeed {
    magnetic_heading: Option<MagneticHeading>,
    airspeed_type: AirspeedType,
    airspeed_value: Option<Velocity>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MagneticHeading(u16);

impl MagneticHeading {
    pub const fn from_u16_unchecked(word: u16) -> Self {
        Self(word)
    }

    pub const fn from_u16(word: u16) -> Option<Self> {
        if word & 0b1111110000000000 == 0 {
            Some(Self(word))
        }
        else {
            None
        }
    }

    /// Magnetic heading as 360/1024 of a degree
    ///
    /// Clockwise from true magnetic north.
    pub fn as_u16(&self) -> u16 {
        self.0
    }

    /// Magnetic heading in degrees
    ///
    /// Clockwise from true magnetic north.
    pub fn as_degrees(&self) -> f64 {
        self.0 as f64 * 360.0 / 1024.0
    }

    /// Magnetic heading in radians
    ///
    /// Clockwise from true magnetic north.
    pub fn as_radians(&self) -> f64 {
        self.0 as f64 * TAU / 1024.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AirspeedType {
    Indicated,
    True,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct VerticalRate {
    pub source: VerticalRateSource,
    pub sign: VerticalRateSign,
    pub value: Option<VerticalRateValue>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum VerticalRateSource {
    Barometric,
    Gnss,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum VerticalRateSign {
    Up,
    Down,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct VerticalRateValue(u16);

impl VerticalRateValue {
    pub const fn from_u16_unchecked(word: u16) -> Self {
        Self(word)
    }

    pub const fn from_u16(word: u16) -> Option<Self> {
        if word & 0b1111111000000000 == 0 && word != 0 {
            Some(Self(word))
        }
        else {
            None
        }
    }

    /// The magnetic heading as 360/1024 of a degree
    pub fn as_u16(&self) -> u16 {
        self.0
    }

    pub fn as_ft_per_min(&self) -> u16 {
        (self.0 - 1) * 64
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AltitudeDifferenceSign {
    GnssAboveBarometric,
    GnssBelowBarometric,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AltitudeDifferenceValue(u8);

impl AltitudeDifferenceValue {
    pub const fn from_u8_unchecked(byte: u8) -> Self {
        Self(byte)
    }

    pub const fn from_u8(byte: u8) -> Option<Self> {
        if byte & 0b10000000 == 0 && byte != 0 {
            Some(Self(byte))
        }
        else {
            None
        }
    }

    pub fn as_u8(&self) -> u8 {
        self.0
    }

    pub fn as_ft(&self) -> u8 {
        (self.0 - 1) * 25
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AltitudeDifference {
    pub sign: AltitudeDifferenceSign,
    pub value: Option<AltitudeDifferenceValue>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NavigationUncertaintyCategory(u8);

impl NavigationUncertaintyCategory {
    pub const fn from_u8_unchecked(byte: u8) -> Self {
        Self(byte)
    }

    pub const fn from_u8(byte: u8) -> Option<Self> {
        if byte & 0b11111000 == 0 && byte != 0 {
            Some(Self(byte))
        }
        else {
            None
        }
    }

    pub fn as_u8(&self) -> u8 {
        self.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TurnIndicator(u8);

impl TurnIndicator {
    pub const fn from_u8_unchecked(byte: u8) -> Self {
        Self(byte)
    }

    pub const fn from_u8(byte: u8) -> Option<Self> {
        if byte & 0b11111000 == 0 && byte != 0 {
            Some(Self(byte))
        }
        else {
            None
        }
    }

    pub fn as_u8(&self) -> u8 {
        self.0
    }
}
