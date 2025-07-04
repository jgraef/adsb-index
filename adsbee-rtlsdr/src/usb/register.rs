use nusb::{
    Interface,
    transfer::{
        ControlIn,
        ControlOut,
        ControlType,
        Recipient,
    },
};

use crate::usb::Error;

pub const BLOCK_DEMOD: u8 = 0;
pub const BLOCK_USB: u8 = 1;
pub const BLOCK_SYS: u8 = 2;
pub const BLOCK_TUN: u8 = 3;
pub const BLOCK_ROM: u8 = 4;
pub const BLOCK_IR: u8 = 5;
pub const BLOCK_IIC: u8 = 6;

pub const USB_SYSCTL: Register = Register {
    block: BLOCK_USB,
    address: 0x2000,
};
pub const USB_EPA_MAXPKT: Register = Register {
    block: BLOCK_USB,
    address: 0x2158,
};
pub const USB_EPA_CTL: Register = Register {
    block: BLOCK_USB,
    address: 0x2148,
};
pub const SYS_DEMOD_CTL: Register = Register {
    block: BLOCK_SYS,
    address: 0x3000,
};
pub const SYS_DEMOD_CTL_1: Register = Register {
    block: BLOCK_SYS,
    address: 0x300b,
};

pub const DEMOD_OPT_ADC_IQ: DemodRegister = DemodRegister {
    page: 0,
    address: 0x06,
};
pub const DEMOD_SDR_MODE: DemodRegister = DemodRegister {
    page: 0,
    address: 0x19,
};

pub const DEMOD_RESET: DemodRegister = DemodRegister {
    page: 0x1,
    address: 0x01,
};
pub const DEMOD_RF_IF_AGC_LOOP: DemodRegister = DemodRegister {
    page: 1,
    address: 0x04,
};
pub const DEMOD_IN_PHASE_ADC: DemodRegister = DemodRegister {
    page: 1,
    address: 0x08,
};
pub const DEMOD_AGC: DemodRegister = DemodRegister {
    page: 1,
    address: 0x11,
};
pub const DEMOD_SPECTRUM_INVERSION: DemodRegister = DemodRegister {
    page: 1,
    address: 0x15,
};
pub const DEMOD_ADJACENT_CHANNEL_REJECTION: DemodRegister = DemodRegister {
    page: 1,
    address: 0x16,
};
pub const DEMOD_PID_FILTER: DemodRegister = DemodRegister {
    page: 1,
    address: 0x61,
};
pub const DEMOD_FSM_STATE_1: DemodRegister = DemodRegister {
    page: 1,
    address: 0x93,
};
pub const DEMOD_FSM_STATE_2: DemodRegister = DemodRegister {
    page: 1,
    address: 0x94,
};
pub const DEMOD_EN: DemodRegister = DemodRegister {
    page: 1,
    address: 0xb1,
};
pub const DEMOD_CLOCK: DemodRegister = DemodRegister {
    page: 1,
    address: 0x0d,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Register {
    pub block: u8,
    pub address: u16,
}

impl Register {
    pub async fn read(&self, interface: &Interface, length: usize) -> Result<Vec<u8>, Error> {
        let index = u16::from(self.block) << 8;

        Ok(interface
            .control_in(ControlIn {
                control_type: ControlType::Vendor,
                recipient: Recipient::Endpoint,
                request: 0,
                value: self.address,
                index,
                length: u16::try_from(length).expect("length must be 16 bit"),
            })
            .await
            .into_result()?)
    }

    pub async fn write(&self, interface: &Interface, data: &[u8]) -> Result<(), Error> {
        let index = (u16::from(self.block) << 8) | 0x10;

        interface
            .control_out(ControlOut {
                control_type: ControlType::Vendor,
                recipient: Recipient::Endpoint,
                request: 0,
                value: self.address,
                index,
                data,
            })
            .await
            .into_result()?;
        Ok(())
    }

    pub async fn read_u8(&self, interface: &Interface) -> Result<u8, Error> {
        let data = self.read(interface, 1).await?;
        Ok(data[0])
    }

    pub async fn read_u16(&self, interface: &Interface) -> Result<u16, Error> {
        let data = self.read(interface, 1).await?;
        Ok(u16::from_be_bytes(data.try_into().unwrap()))
    }

    pub async fn write_u8(&self, interface: &Interface, value: u8) -> Result<(), Error> {
        self.write(interface, &[value]).await?;
        Ok(())
    }

    pub async fn write_u16(&self, interface: &Interface, value: u16) -> Result<(), Error> {
        let data = value.to_be_bytes();
        self.write(interface, &data).await?;
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DemodRegister {
    pub page: u8,
    pub address: u8,
}

impl DemodRegister {
    // this is only used in Self::read??
    async fn read(&self, interface: &Interface, length: usize) -> Result<Vec<u8>, Error> {
        let address = u16::from(self.address) << 8 | 0x20;
        let index = u16::from(self.page);

        Ok(interface
            .control_in(ControlIn {
                control_type: ControlType::Vendor,
                recipient: Recipient::Endpoint,
                request: 0,
                value: address,
                index,
                length: u16::try_from(length).expect("length must be 16 bit"),
            })
            .await
            .into_result()?)
    }

    pub async fn write(&self, interface: &Interface, data: &[u8]) -> Result<(), Error> {
        let address = (u16::from(self.address) << 8) | 0x20;
        let index = u16::from(self.page);

        interface
            .control_out(ControlOut {
                control_type: ControlType::Vendor,
                recipient: Recipient::Endpoint,
                request: 0,
                value: address,
                index,
                data,
            })
            .await
            .into_result()?;

        // todo: what for?
        let _ = Self {
            page: 0x0a,
            address: 0x01,
        }
        .read(interface, 1)
        .await?;

        Ok(())
    }

    pub async fn write_u8(&self, interface: &Interface, value: u8) -> Result<(), Error> {
        self.write(interface, &[value]).await?;
        Ok(())
    }

    pub async fn write_u16(&self, interface: &Interface, value: u16) -> Result<(), Error> {
        let data = value.to_be_bytes();
        self.write(interface, &data).await?;
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct I2cRegister {
    pub address: u8,
    pub register: u8,
}

impl I2cRegister {
    pub async fn read_u8(&self, interface: &Interface) -> Result<u8, Error> {
        let register = Register {
            block: BLOCK_IIC,
            address: self.address.into(),
        };
        register.write_u8(interface, self.register).await?;
        register.read_u8(interface).await
    }

    pub async fn write_u8(&self, interface: &Interface, value: u8) -> Result<(), Error> {
        let data = [self.register, value];
        let register = Register {
            block: BLOCK_IIC,
            address: self.address.into(),
        };
        register.write(interface, &data).await
    }
}
