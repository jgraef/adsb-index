mod r82xx;

use crate::usb::{
    Config,
    DeviceInfo,
    Error,
    Interface,
    register::I2cRegister,
};

#[derive(Clone, Copy, Debug)]
pub struct TunerProbe {
    pub tuner_type: TunerType,
    pub register: I2cRegister,
    pub bitmask: u8,
    pub expected_value: u8,
    pub gpio: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TunerType {
    E4000,
    Fc0012,
    Fc0013,
    Fc2580,
    R820t,
    R828d,
}

pub const TUNER_PROBES: &[TunerProbe] = &[TunerProbe {
    tuner_type: TunerType::R828d,
    register: I2cRegister {
        address: r82xx::R828D_I2C_ADDRESS,
        register: 0x00,
    },
    bitmask: 0xff,
    expected_value: 0x69,
    gpio: false,
}];

#[derive(Debug)]
pub enum TunerImpl {
    R820t(r82xx::R82xx),
}

impl TunerImpl {
    pub async fn init(
        tuner_type: TunerType,
        device_info: &DeviceInfo,
        config: &mut Config,
        interface: &Interface,
    ) -> Result<Self, Error> {
        match tuner_type {
            TunerType::R828d => {
                Ok(Self::R820t(
                    r82xx::R82xx::new(tuner_type, device_info, config, interface).await?,
                ))
            }
            _ => todo!("{tuner_type:?}"),
        }
    }
}
