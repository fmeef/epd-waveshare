//! A Driver for the Waveshare 2.13" E-Ink Display (V2) via SPI
//!
//! # References
//!
//! - [Waveshare product page](https://www.waveshare.com/wiki/2.13inch_e-Paper_HAT)
//! - [Waveshare C driver](https://github.com/waveshare/e-Paper/blob/master/RaspberryPi%26JetsonNano/c/lib/e-Paper/EPD_2in13_V2.c)
//! - [Waveshare Python driver](https://github.com/waveshare/e-Paper/blob/master/RaspberryPi%26JetsonNano/python/lib/waveshare_epd/epd2in13_V2.py)
//! - [Controller Datasheet SS1780](http://www.e-paper-display.com/download_detail/downloadsId=682.html)
//!
extern crate std;
use std::println;

use embedded_hal::{
    blocking::{delay::*, spi::Write},
    digital::v2::{InputPin, OutputPin},
};

use crate::buffer_len;
use crate::color::Color;
use crate::interface::DisplayInterface;
use crate::traits::{InternalWiAdditions, RefreshLut, WaveshareDisplay};

pub(crate) mod command;
pub(crate) mod constants;

use self::command::{Command, DriverOutput, I32Ext, Vcom};

#[cfg(feature = "graphics")]
mod graphics;
#[cfg(feature = "graphics")]
pub use self::graphics::Display2in13;

/// Width of the display.
pub const WIDTH: u32 = 104;

/// Height of the display
pub const HEIGHT: u32 = 212;

/// Default Background Color
pub const DEFAULT_BACKGROUND_COLOR: Color = Color::White;
const IS_BUSY_LOW: bool = false;

/// Epd2in13 (V2) driver
///
pub struct Epd2in13<SPI, CS, BUSY, DC, RST, DELAY> {
    /// Connection Interface
    interface: DisplayInterface<SPI, CS, BUSY, DC, RST, DELAY>,

    /// Background Color
    background_color: Color,
    refresh: RefreshLut,
}

impl<SPI, CS, BUSY, DC, RST, DELAY> InternalWiAdditions<SPI, CS, BUSY, DC, RST, DELAY>
    for Epd2in13<SPI, CS, BUSY, DC, RST, DELAY>
where
    SPI: Write<u8>,
    CS: OutputPin,
    BUSY: InputPin,
    DC: OutputPin,
    RST: OutputPin,
    DELAY: DelayMs<u8>,
{
    fn init(&mut self, spi: &mut SPI, delay: &mut DELAY) -> Result<(), SPI::Error> {
        // HW reset
        println!("init");
        self.interface.reset(delay, 10);
        delay.delay_ms(10);
        println!("reset");
        self.wait_until_idle();
        println!("not busy");
        self.set_lut(spi, None)?;
        println!("set lut");
        self.wait_until_idle();
        println!("not busy");
        self.set_driver_output(
            spi,
            DriverOutput {
                scan_is_linear: true,
                scan_g0_is_first: true,
                scan_dir_incr: false,
                width: (HEIGHT - 1) as u16,
            },
        )?;

        println!("set driver output");
        self.wait_until_idle();
        println!("not busy");
        self.set_vcom_register(spi)?;
        self.wait_until_idle();
        println!("success!");
        Ok(())
    }
}

impl<SPI, CS, BUSY, DC, RST, DELAY> WaveshareDisplay<SPI, CS, BUSY, DC, RST, DELAY>
    for Epd2in13<SPI, CS, BUSY, DC, RST, DELAY>
where
    SPI: Write<u8>,
    CS: OutputPin,
    BUSY: InputPin,
    DC: OutputPin,
    RST: OutputPin,
    DELAY: DelayMs<u8>,
{
    type DisplayColor = Color;
    fn new(
        spi: &mut SPI,
        cs: CS,
        busy: BUSY,
        dc: DC,
        rst: RST,
        delay: &mut DELAY,
    ) -> Result<Self, SPI::Error> {
        let mut epd = Epd2in13 {
            interface: DisplayInterface::new(cs, busy, dc, rst),
            background_color: DEFAULT_BACKGROUND_COLOR,
            refresh: RefreshLut::Full,
        };

        epd.init(spi, delay)?;
        Ok(epd)
    }

    fn wake_up(&mut self, spi: &mut SPI, delay: &mut DELAY) -> Result<(), SPI::Error> {
        self.init(spi, delay)
    }

    fn sleep(&mut self, spi: &mut SPI, _delay: &mut DELAY) -> Result<(), SPI::Error> {
        self.wait_until_idle();
        self.cmd_with_data(spi, Command::WriteVcomRegister, &[0xf7 as u8])?;
        self.wait_until_idle();
        self.command(spi, Command::PowerOff)?;
        self.cmd_with_data(spi, Command::DeepSleep, &[0xa5 as u8])
    }

    fn update_frame(
        &mut self,
        spi: &mut SPI,
        buffer: &[u8],
        _delay: &mut DELAY,
    ) -> Result<(), SPI::Error> {
        assert!(buffer.len() == buffer_len(WIDTH as usize, HEIGHT as usize));
        self.cmd_with_data(spi, Command::WriteRam, buffer)?;

        println!("writeram");
        //   self.cmd_with_data(spi, Command::WriteRamRed, buffer)?;
        println!("writeram red");
        Ok(())
    }

    /// Updating only a part of the frame is not supported when using the
    /// partial refresh feature. The function will panic if called when set to
    /// use partial refresh.
    fn update_partial_frame(
        &mut self,
        spi: &mut SPI,
        buffer: &[u8],
        _x: u32,
        _y: u32,
        width: u32,
        height: u32,
    ) -> Result<(), SPI::Error> {
        assert!((width * height / 8) as usize == buffer.len());

        // This should not be used when doing partial refresh. The RAM_RED must
        // be updated with the last buffer having been displayed. Doing partial
        // update directly in RAM makes this update impossible (we can't read
        // RAM content). Using this function will most probably make the actual
        // display incorrect as the controler will compare with something
        // incorrect.
        assert!(self.refresh == RefreshLut::Full);

        self.cmd_with_data(spi, Command::WriteRam, buffer)?;

        // self.cmd_with_data(spi, Command::WriteRamRed, buffer)?;

        Ok(())
    }

    /// Never use directly this function when using partial refresh, or also
    /// keep the base buffer in syncd using `set_partial_base_buffer` function.
    fn display_frame(&mut self, spi: &mut SPI, _delay: &mut DELAY) -> Result<(), SPI::Error> {
        self.wait_until_idle();
        self.command(spi, Command::DisplayRefresh)?;
        self.wait_until_idle();
        Ok(())
    }

    fn update_and_display_frame(
        &mut self,
        spi: &mut SPI,
        buffer: &[u8],
        delay: &mut DELAY,
    ) -> Result<(), SPI::Error> {
        self.update_frame(spi, buffer, delay)?;
        println!("update frame");
        self.display_frame(spi, delay)?;
        println!("display frame");
        Ok(())
    }

    fn clear_frame(&mut self, spi: &mut SPI, _delay: &mut DELAY) -> Result<(), SPI::Error> {
        let color = self.background_color.get_byte_value();

        self.command(spi, Command::WriteRam)?;
        self.interface.data_x_times(
            spi,
            color,
            buffer_len(WIDTH as usize, HEIGHT as usize) as u32,
        )?;

        //self.command(spi, Command::WriteRamRed)?;
        /*
        self.interface.data_x_times(
            spi,
            color,
            buffer_len(WIDTH as usize, HEIGHT as usize) as u32,
        )?;
        */
        Ok(())
    }

    fn set_background_color(&mut self, background_color: Color) {
        self.background_color = background_color;
    }

    fn background_color(&self) -> &Color {
        &self.background_color
    }

    fn width(&self) -> u32 {
        WIDTH
    }

    fn height(&self) -> u32 {
        HEIGHT
    }

    fn set_lut(
        &mut self,
        spi: &mut SPI,
        _refresh_rate: Option<RefreshLut>,
    ) -> Result<(), SPI::Error> {
        self.cmd_with_data(spi, Command::WriteLutRegister, &constants::LUT_OTP)
    }

    fn is_busy(&self) -> bool {
        self.interface.is_busy(IS_BUSY_LOW)
    }
}

impl<SPI, CS, BUSY, DC, RST, DELAY> Epd2in13<SPI, CS, BUSY, DC, RST, DELAY>
where
    SPI: Write<u8>,
    CS: OutputPin,
    BUSY: InputPin,
    DC: OutputPin,
    RST: OutputPin,
    DELAY: DelayMs<u8>,
{
    /// Sets the refresh mode. When changing mode, the screen will be
    /// re-initialized accordingly.
    pub fn set_refresh(
        &mut self,
        spi: &mut SPI,
        delay: &mut DELAY,
        refresh: RefreshLut,
    ) -> Result<(), SPI::Error> {
        if self.refresh != refresh {
            self.refresh = refresh;
            self.init(spi, delay)?;
        }
        Ok(())
    }

    fn set_vcom_register(&mut self, spi: &mut SPI) -> Result<(), SPI::Error> {
        self.cmd_with_data(spi, Command::WriteVcomRegister, &[0x77 as u8])
    }

    fn set_driver_output(&mut self, spi: &mut SPI, output: DriverOutput) -> Result<(), SPI::Error> {
        self.cmd_with_data(spi, Command::DriverOutputControl, &[0x68, 0x00, 0xd4])
    }

    fn command(&mut self, spi: &mut SPI, command: Command) -> Result<(), SPI::Error> {
        self.interface.cmd(spi, command)
    }

    fn cmd_with_data(
        &mut self,
        spi: &mut SPI,
        command: Command,
        data: &[u8],
    ) -> Result<(), SPI::Error> {
        self.interface.cmd_with_data(spi, command, data)
    }

    fn wait_until_idle(&mut self) {
        let _ = self.interface.wait_until_idle(IS_BUSY_LOW);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn epd_size() {
        assert_eq!(WIDTH, 122);
        assert_eq!(HEIGHT, 250);
        assert_eq!(DEFAULT_BACKGROUND_COLOR, Color::White);
    }
}
