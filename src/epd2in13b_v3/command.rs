//! SPI Commands for the Waveshare 2.13" v2

use crate::traits;

extern crate bit_field;
use bit_field::BitField;

/// Epd2in13 v2
///
/// For more infos about the addresses and what they are doing look into the pdfs
#[allow(dead_code)]
#[derive(Copy, Clone)]
pub(crate) enum Command {
    DriverOutputControl = 0x61,
    WriteVcomRegister = 0x50,
    WriteRamRed = 0x13,
    WriteRam = 0x10,
    WriteLutRegister = 0x00,
    DisplayRefresh = 0x12,
    PowerOff = 0x02,
    DeepSleep = 0x07,
}

pub(crate) struct DriverOutput {
    pub scan_is_linear: bool,
    pub scan_g0_is_first: bool,
    pub scan_dir_incr: bool,

    pub width: u16,
}

impl DriverOutput {
    pub fn to_bytes(&self) -> [u8; 3] {
        [
            self.width as u8,
            (self.width >> 8) as u8,
            *0u8.set_bit(0, !self.scan_dir_incr)
                .set_bit(1, !self.scan_g0_is_first)
                .set_bit(2, !self.scan_is_linear),
        ]
    }
}

/// These are not directly documented, but the bitfield is easily reversed from
/// documentation and sample code
/// [7|6|5|4|3|2|1|0]
///  | | | | | | | `--- disable clock
///  | | | | | | `----- disable analog
///  | | | | | `------- display
///  | | | | `--------- undocumented and unknown use,
///  | | | |            but used in waveshare reference code
///  | | | `----------- load LUT
///  | | `------------- load temp
///  | `--------------- enable clock
///  `----------------- enable analog

#[allow(dead_code, clippy::enum_variant_names)]
pub(crate) enum DataEntryModeIncr {
    XDecrYDecr = 0x0,
    XIncrYDecr = 0x1,
    XDecrYIncr = 0x2,
    XIncrYIncr = 0x3,
}

#[allow(dead_code)]
pub(crate) enum DataEntryModeDir {
    XDir = 0x0,
    YDir = 0x4,
}

#[allow(dead_code)]
#[derive(Copy, Clone)]
pub(crate) enum BorderWaveFormVbd {
    Gs = 0x0,
    FixLevel = 0x1,
    Vcom = 0x2,
}

#[allow(dead_code)]
#[derive(Copy, Clone)]
pub(crate) enum BorderWaveFormFixLevel {
    Vss = 0x0,
    Vsh1 = 0x1,
    Vsl = 0x2,
    Vsh2 = 0x3,
}

#[allow(dead_code)]
#[derive(Copy, Clone)]
pub(crate) enum BorderWaveFormGs {
    Lut0 = 0x0,
    Lut1 = 0x1,
    Lut2 = 0x2,
    Lut3 = 0x3,
}

pub(crate) struct GateDrivingVoltage(pub u8);
pub(crate) struct SourceDrivingVoltage(pub u8);
pub(crate) struct Vcom(pub u8);

pub(crate) trait I32Ext {
    fn vcom(self) -> Vcom;
    fn gate_driving_decivolt(self) -> GateDrivingVoltage;
    fn source_driving_decivolt(self) -> SourceDrivingVoltage;
}

impl I32Ext for i32 {
    // This is really not very nice. Until I find something better, this will be
    // a placeholder.
    fn vcom(self) -> Vcom {
        assert!((-30..=-2).contains(&self));
        let u = match -self {
            2 => 0x08,
            3 => 0x0B,
            4 => 0x10,
            5 => 0x14,
            6 => 0x17,
            7 => 0x1B,
            8 => 0x20,
            9 => 0x24,
            10 => 0x28,
            11 => 0x2C,
            12 => 0x2F,
            13 => 0x34,
            14 => 0x37,
            15 => 0x3C,
            16 => 0x40,
            17 => 0x44,
            18 => 0x48,
            19 => 0x4B,
            20 => 0x50,
            21 => 0x54,
            22 => 0x58,
            23 => 0x5B,
            24 => 0x5F,
            25 => 0x64,
            26 => 0x68,
            27 => 0x6C,
            28 => 0x6F,
            29 => 0x73,
            30 => 0x78,
            _ => 0,
        };
        Vcom(u)
    }

    fn gate_driving_decivolt(self) -> GateDrivingVoltage {
        assert!((100..=210).contains(&self) && self % 5 == 0);
        GateDrivingVoltage(((self - 100) / 5 + 0x03) as u8)
    }

    fn source_driving_decivolt(self) -> SourceDrivingVoltage {
        assert!((24..=88).contains(&self) || (self % 5 == 0 && (90..=180).contains(&self.abs())));

        if (24..=88).contains(&self) {
            SourceDrivingVoltage(((self - 24) + 0x8E) as u8)
        } else if (90..=180).contains(&self) {
            SourceDrivingVoltage(((self - 90) / 2 + 0x23) as u8)
        } else {
            SourceDrivingVoltage((((-self - 90) / 5) * 2 + 0x1A) as u8)
        }
    }
}

impl traits::Command for Command {
    /// Returns the address of the command
    fn address(self) -> u8 {
        self as u8
    }
}
