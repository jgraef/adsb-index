mod known_devices;
mod register;
mod tuner;

use nusb::transfer::TransferError;

pub use crate::usb::tuner::TunerType;
use crate::usb::{
    known_devices::KnownDevice,
    register::{
        DEMOD_ADJACENT_CHANNEL_REJECTION,
        DEMOD_AGC,
        DEMOD_CLOCK,
        DEMOD_EN,
        DEMOD_FSM_STATE_1,
        DEMOD_FSM_STATE_2,
        DEMOD_OPT_ADC_IQ,
        DEMOD_PID_FILTER,
        DEMOD_RESET,
        DEMOD_RF_IF_AGC_LOOP,
        DEMOD_SDR_MODE,
        DEMOD_SPECTRUM_INVERSION,
        DemodRegister,
        SYS_DEMOD_CTL,
        SYS_DEMOD_CTL_1,
        USB_EPA_CTL,
        USB_EPA_MAXPKT,
        USB_SYSCTL,
    },
    tuner::{
        TUNER_PROBES,
        TunerImpl,
    },
};

#[derive(Debug, thiserror::Error)]
#[error("rtlsdr usb error")]
pub enum Error {
    Usb(#[from] nusb::Error),
    InvalidFir(#[from] FirEncodeError),
    NoTunerFound,
}

impl From<TransferError> for Error {
    fn from(value: TransferError) -> Self {
        nusb::Error::from(value).into()
    }
}

/// Lists available RTL-SDR devices.
pub fn list_devices() -> Result<impl Iterator<Item = DeviceInfo>, Error> {
    Ok(nusb::list_devices()?.filter_map(|device_info| {
        let known_device =
            known_devices::lookup(device_info.vendor_id(), device_info.product_id())?;

        Some(DeviceInfo {
            known_device,
            device_info,
        })
    }))
}

/// [This][1] always uses interface 0
///
/// [1]: https://github.com/rtlsdrblog/rtl-sdr-blog/blob/master/src/librtlsdr.c
const INTERFACE: u8 = 0;

const DEFAULT_BUF_NUMBER: usize = 15;
const DEFAULT_BUF_LENGTH: usize = 0x40000;
const DEFAULT_RTL_XTAL_FREQ: u32 = 28800000;
const MIN_RTL_XTAL_FREQ: u32 = DEFAULT_RTL_XTAL_FREQ - 1000;
const MAX_RTL_XTAL_FREQ: u32 = DEFAULT_RTL_XTAL_FREQ + 1000;

/// # TODO
///
/// Getter for name
#[derive(Clone, Debug)]
pub struct DeviceInfo {
    known_device: &'static KnownDevice,
    device_info: nusb::DeviceInfo,
}

impl DeviceInfo {
    pub async fn open(&self) -> Result<Device, Error> {
        let device = self.device_info.open()?;

        // todo: how do we know if it was attached, so that we can reattach it later?
        //device.detach_kernel_driver(INTERFACE);

        let interface = Interface {
            interface: device.claim_interface(INTERFACE)?,
        };

        let config = Config {
            rate: todo!(),
            rtl_xtal: DEFAULT_RTL_XTAL_FREQ,
            tuner_xtal: DEFAULT_RTL_XTAL_FREQ,
            frequency_correction: 0,
            fir: Fir::DEFAULT,
        };

        // perform a dummy write, if it fails, reset the device
        if let Err(error) = interface.dummy_write().await {
            device.reset()?;
        }

        // init baseband
        interface.init_baseband(&config.fir).await;

        let tuner_type = interface
            .probe_tuners()
            .await?
            .ok_or_else(|| Error::NoTunerFound)?;
        let tuner_impl = TunerImpl::init(tuner_type, self, &mut config, &interface).await?;

        Ok(Device {
            device_info: self.clone(),
            interface,
            config,
        })
    }
}

#[derive(Debug)]
pub struct Device {
    device_info: DeviceInfo,
    interface: Interface,
    config: Config,
}

#[derive(Debug)]
pub(crate) struct Config {
    pub rate: u32,                 // Hz
    pub rtl_xtal: u32,             // Hz
    pub tuner_xtal: u32,           // Hz
    pub frequency_correction: i32, // in ppm
    pub fir: Fir,
}

impl Config {
    pub fn corrected_rtl_frequency(&self) -> u32 {
        corrected_frequency(self.rtl_xtal, self.frequency_correction)
    }

    pub fn corrected_tuner_frequency(&self) -> u32 {
        corrected_frequency(self.rtl_xtal, self.frequency_correction)
    }
}

fn corrected_frequency(frequency: u32, correction: i32) -> u32 {
    if correction == 0 {
        frequency
    }
    else {
        (frequency as f32 * (1.0 + correction as f32)) as u32
    }
}

#[derive(derive_more::Debug)]
struct Interface {
    #[debug(skip)]
    interface: nusb::Interface,
}

impl Interface {
    async fn dummy_write(&self) -> Result<(), Error> {
        USB_SYSCTL.write_u8(&self.interface, 0x09).await
    }

    async fn init_baseband(&self, fir: &Fir) -> Result<(), Error> {
        // initialize USB
        USB_SYSCTL.write_u8(&self.interface, 0x09).await?;
        USB_EPA_MAXPKT.write_u16(&self.interface, 0x0002).await?;
        USB_EPA_CTL.write_u16(&self.interface, 0x1002).await?;

        // poweron demod
        SYS_DEMOD_CTL_1.write_u8(&self.interface, 0x22).await?;
        SYS_DEMOD_CTL.write_u8(&self.interface, 0xe8).await?;

        // reset demod
        DEMOD_RESET.write_u8(&self.interface, 0x14).await?;
        DEMOD_RESET.write_u8(&self.interface, 0x10).await?;

        // disable spectrum inversion and adjacent channel rejection
        DEMOD_SPECTRUM_INVERSION
            .write_u8(&self.interface, 0x00)
            .await?;
        DEMOD_ADJACENT_CHANNEL_REJECTION
            .write_u16(&self.interface, 0x0000)
            .await?;

        // clear both DDC shift and IF frequency registers
        for i in 0..6 {
            DemodRegister {
                page: 1,
                address: 0x16 + i,
            }
            .write_u8(&self.interface, 0x00)
            .await?;
        }
        self.set_fir(fir).await?;

        // enable SDR mode, disable DAGC (bit 5)
        DEMOD_SDR_MODE.write_u8(&self.interface, 0x05).await?;

        // init FSM state-holding register
        DEMOD_FSM_STATE_1.write_u8(&self.interface, 0xf0).await?;
        DEMOD_FSM_STATE_2.write_u8(&self.interface, 0x0f).await?;

        // disable AGC (en_dagc, bit 0) (this seems to have no effect)
        DEMOD_AGC.write_u8(&self.interface, 0x00).await?;

        // disable RF and IF AGC loop
        DEMOD_RF_IF_AGC_LOOP.write_u8(&self.interface, 0x00).await?;

        // disable PID filter (enable_PID = 0)
        DEMOD_PID_FILTER.write_u8(&self.interface, 0x60).await?;

        // opt_adc_iq = 0, default ADC_I/ADC_Q datapath
        DEMOD_OPT_ADC_IQ.write_u8(&self.interface, 0x80).await?;

        // Enable Zero-IF mode (en_bbin bit), DC cancellation (en_dc_est),
        // IQ estimation/compensation (en_iq_comp, en_iq_est) */
        DEMOD_EN.write_u8(&self.interface, 0x1b).await?;

        // disable 4.096 MHz clock output on pin TP_CK0
        DEMOD_CLOCK.write_u8(&self.interface, 0x83).await?;

        Ok(())
    }

    async fn set_fir(&self, fir: &Fir) -> Result<(), Error> {
        let mut buf = [0u8; 20];
        fir.encode(&mut buf)?;

        for i in 0..20 {
            DemodRegister {
                page: 1,
                address: 0x1c + u8::try_from(i).unwrap(),
            }
            .write_u8(&self.interface, buf[i])
            .await?;
        }

        todo!();
    }

    async fn set_i2c_repeater(&self, enable: bool) -> Result<(), Error> {
        const DEMOD_I2C_REPEATER: DemodRegister = DemodRegister {
            page: 1,
            address: 0x01,
        };
        DEMOD_I2C_REPEATER
            .write_u8(&self.interface, if enable { 0x18 } else { 0x10 })
            .await?;
        Ok(())
    }

    async fn probe_tuners(&self) -> Result<Option<TunerType>, Error> {
        self.set_i2c_repeater(true).await?;

        for probe in TUNER_PROBES {
            if probe.gpio {
                todo!();
            }

            let value = probe.register.read_u8(&self.interface).await?;
            if value & probe.bitmask == probe.expected_value {
                return Ok(Some(probe.tuner_type));
            }
        }

        Ok(None)
    }

    async fn set_if_frequency(&self, frequency: u32) -> Result<(), Error> {
        todo!();
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Fir(pub [i16; Self::LENGTH]);

impl Fir {
    pub const LENGTH: usize = 16;
    pub const DEFAULT: Self = Self([
        -54, -36, -41, -40, -32, -14, 14, 53, 101, 156, 215, 273, 327, 372, 404, 421,
    ]);

    fn encode(&self, output: &mut [u8; 20]) -> Result<(), FirEncodeError> {
        for i in 0..8 {
            let value = self.0[i];
            if value < -128 || value > 127 {
                return Err(FirEncodeError { index: i, value });
            }
            output[i] = value as u8;
        }

        let mut i = 8;
        let mut j = 8;
        while i < 8 {
            let value0 = self.0[i];
            let value1 = self.0[i + 1];

            if value0 < -2048 || value0 > 2047 {
                return Err(FirEncodeError {
                    index: i,
                    value: value0,
                });
            }
            if value1 < -2048 || value1 > 2047 {
                return Err(FirEncodeError {
                    index: i + 1,
                    value: value1,
                });
            }

            output[j] = (value0 >> 4) as u8;
            output[j + 1] = ((value0 << 4) | ((value1 >> 8) & 0x0f)) as u8;
            output[j + 2] = value1 as u8;

            i += 2;
            j += 3;
        }

        Ok(())
    }
}

impl Default for Fir {
    fn default() -> Self {
        Self::DEFAULT
    }
}

#[derive(Debug, thiserror::Error)]
#[error("Invalid FIR value {value} at index {index}")]
pub struct FirEncodeError {
    index: usize,
    value: i16,
}
