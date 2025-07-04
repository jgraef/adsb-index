use std::{
    collections::HashMap,
    sync::OnceLock,
};

#[derive(Clone, Copy, Debug)]
pub struct KnownDevice {
    pub vendor_id: u16,
    pub product_id: u16,
    pub name: &'static str,
}

/// <https://github.com/rtlsdrblog/rtl-sdr-blog/blob/240bd0e1e6d9f64361b6949047468958cd08aa31/src/librtlsdr.c#L311>
pub const KNOWN_DEVICES: &'static [KnownDevice] = &[
    KnownDevice {
        vendor_id: 0x0bda,
        product_id: 0x2832,
        name: "Generic RTL2832U",
    },
    KnownDevice {
        vendor_id: 0x0bda,
        product_id: 0x2838,
        name: "Generic RTL2832U OEM",
    },
    KnownDevice {
        vendor_id: 0x0413,
        product_id: 0x6680,
        name: "DigitalNow Quad DVB-T PCI-E card",
    },
    KnownDevice {
        vendor_id: 0x0413,
        product_id: 0x6f0f,
        name: "Leadtek WinFast DTV Dongle mini D",
    },
    KnownDevice {
        vendor_id: 0x0458,
        product_id: 0x707f,
        name: "Genius TVGo DVB-T03 USB dongle (Ver. B)",
    },
    KnownDevice {
        vendor_id: 0x0ccd,
        product_id: 0x00a9,
        name: "Terratec Cinergy T Stick Black (rev 1)",
    },
    KnownDevice {
        vendor_id: 0x0ccd,
        product_id: 0x00b3,
        name: "Terratec NOXON DAB/DAB+ USB dongle (rev 1)",
    },
    KnownDevice {
        vendor_id: 0x0ccd,
        product_id: 0x00b4,
        name: "Terratec Deutschlandradio DAB Stick",
    },
    KnownDevice {
        vendor_id: 0x0ccd,
        product_id: 0x00b5,
        name: "Terratec NOXON DAB Stick - Radio Energy",
    },
    KnownDevice {
        vendor_id: 0x0ccd,
        product_id: 0x00b7,
        name: "Terratec Media Broadcast DAB Stick",
    },
    KnownDevice {
        vendor_id: 0x0ccd,
        product_id: 0x00b8,
        name: "Terratec BR DAB Stick",
    },
    KnownDevice {
        vendor_id: 0x0ccd,
        product_id: 0x00b9,
        name: "Terratec WDR DAB Stick",
    },
    KnownDevice {
        vendor_id: 0x0ccd,
        product_id: 0x00c0,
        name: "Terratec MuellerVerlag DAB Stick",
    },
    KnownDevice {
        vendor_id: 0x0ccd,
        product_id: 0x00c6,
        name: "Terratec Fraunhofer DAB Stick",
    },
    KnownDevice {
        vendor_id: 0x0ccd,
        product_id: 0x00d3,
        name: "Terratec Cinergy T Stick RC (Rev.3)",
    },
    KnownDevice {
        vendor_id: 0x0ccd,
        product_id: 0x00d7,
        name: "Terratec T Stick PLUS",
    },
    KnownDevice {
        vendor_id: 0x0ccd,
        product_id: 0x00e0,
        name: "Terratec NOXON DAB/DAB+ USB dongle (rev 2)",
    },
    KnownDevice {
        vendor_id: 0x1554,
        product_id: 0x5020,
        name: "PixelView PV-DT235U(RN)",
    },
    KnownDevice {
        vendor_id: 0x15f4,
        product_id: 0x0131,
        name: "Astrometa DVB-T/DVB-T2",
    },
    KnownDevice {
        vendor_id: 0x15f4,
        product_id: 0x0133,
        name: "HanfTek DAB+FM+DVB-T",
    },
    KnownDevice {
        vendor_id: 0x185b,
        product_id: 0x0620,
        name: "Compro Videomate U620F",
    },
    KnownDevice {
        vendor_id: 0x185b,
        product_id: 0x0650,
        name: "Compro Videomate U650F",
    },
    KnownDevice {
        vendor_id: 0x185b,
        product_id: 0x0680,
        name: "Compro Videomate U680F",
    },
    KnownDevice {
        vendor_id: 0x1b80,
        product_id: 0xd393,
        name: "GIGABYTE GT-U7300",
    },
    KnownDevice {
        vendor_id: 0x1b80,
        product_id: 0xd394,
        name: "DIKOM USB-DVBT HD",
    },
    KnownDevice {
        vendor_id: 0x1b80,
        product_id: 0xd395,
        name: "Peak 102569AGPK",
    },
    KnownDevice {
        vendor_id: 0x1b80,
        product_id: 0xd397,
        name: "KWorld KW-UB450-T USB DVB-T Pico TV",
    },
    KnownDevice {
        vendor_id: 0x1b80,
        product_id: 0xd398,
        name: "Zaapa ZT-MINDVBZP",
    },
    KnownDevice {
        vendor_id: 0x1b80,
        product_id: 0xd39d,
        name: "SVEON STV20 DVB-T USB & FM",
    },
    KnownDevice {
        vendor_id: 0x1b80,
        product_id: 0xd3a4,
        name: "Twintech UT-40",
    },
    KnownDevice {
        vendor_id: 0x1b80,
        product_id: 0xd3a8,
        name: "ASUS U3100MINI_PLUS_V2",
    },
    KnownDevice {
        vendor_id: 0x1b80,
        product_id: 0xd3af,
        name: "SVEON STV27 DVB-T USB & FM",
    },
    KnownDevice {
        vendor_id: 0x1b80,
        product_id: 0xd3b0,
        name: "SVEON STV21 DVB-T USB & FM",
    },
    KnownDevice {
        vendor_id: 0x1d19,
        product_id: 0x1101,
        name: "Dexatek DK DVB-T Dongle (Logilink VG0002A)",
    },
    KnownDevice {
        vendor_id: 0x1d19,
        product_id: 0x1102,
        name: "Dexatek DK DVB-T Dongle (MSI DigiVox mini II V3.0)",
    },
    KnownDevice {
        vendor_id: 0x1d19,
        product_id: 0x1103,
        name: "Dexatek Technology Ltd. DK 5217 DVB-T Dongle",
    },
    KnownDevice {
        vendor_id: 0x1d19,
        product_id: 0x1104,
        name: "MSI DigiVox Micro HD",
    },
    KnownDevice {
        vendor_id: 0x1f4d,
        product_id: 0xa803,
        name: "Sweex DVB-T USB",
    },
    KnownDevice {
        vendor_id: 0x1f4d,
        product_id: 0xb803,
        name: "GTek T803",
    },
    KnownDevice {
        vendor_id: 0x1f4d,
        product_id: 0xc803,
        name: "Lifeview LV5TDeluxe",
    },
    KnownDevice {
        vendor_id: 0x1f4d,
        product_id: 0xd286,
        name: "MyGica TD312",
    },
    KnownDevice {
        vendor_id: 0x1f4d,
        product_id: 0xd803,
        name: "PROlectrix DV107669",
    },
];

fn hash_map() -> &'static HashMap<(u16, u16), &'static KnownDevice> {
    static HASH_MAP: OnceLock<HashMap<(u16, u16), &'static KnownDevice>> = OnceLock::new();
    HASH_MAP.get_or_init(|| {
        let mut hash_map = HashMap::with_capacity(KNOWN_DEVICES.len());
        for known_device in KNOWN_DEVICES {
            hash_map.insert(
                (known_device.vendor_id, known_device.product_id),
                known_device,
            );
        }
        hash_map
    })
}

pub fn lookup(vendor_id: u16, product_id: u16) -> Option<&'static KnownDevice> {
    hash_map().get(&(vendor_id, product_id)).copied()
}
