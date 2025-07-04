use crate::usb::{
    Config,
    DeviceInfo,
    Error,
    Interface,
    TunerType,
    register::{
        DEMOD_EN,
        DEMOD_IN_PHASE_ADC,
        DEMOD_SPECTRUM_INVERSION,
    },
};

pub const R828D_I2C_ADDRESS: u8 = 0x74;
pub const R828D_XTAL_FREQ: u32 = 16_000_000;
pub const R82XX_IF_FREQ: u32 = 3_570_000;

#[derive(Debug)]
pub struct R82xx {
    // todo
}

impl R82xx {
    pub async fn new(
        tuner_type: TunerType,
        device_info: &DeviceInfo,
        config: &mut Config,
        interface: &Interface,
    ) -> Result<Self, Error> {
        if tuner_type == TunerType::R828d
            && device_info
                .device_info
                .manufacturer_string()
                .map_or(false, |s| s == "RTLSDRBlog")
            && device_info
                .device_info
                .product_string()
                .map_or(false, |s| s == "Blog V4")
        {
            config.tuner_xtal = R828D_XTAL_FREQ;
        }

        // disable Zero-IF mode
        DEMOD_EN.write_u8(&interface.interface, 0x1a).await?;

        // only enable In-phase ADC input
        DEMOD_IN_PHASE_ADC
            .write_u8(&interface.interface, 0x4d)
            .await?;

        // the R82XX use 3.57 MHz IF for the DVB-T 6 MHz mode, and 4.57 MHz for the 8
        // MHz mode
        interface.set_if_frequency(R82XX_IF_FREQ).await?;

        /* enable spectrum inversion */
        DEMOD_SPECTRUM_INVERSION
            .write_u8(&interface.interface, 0x01)
            .await?;

        todo!();
    }
}
