#![no_std]
#![no_main]

use core::fmt::Write;

// pick a panicking behavior
use panic_halt as _;

use cortex_m_rt::entry;
// use cortex_m_semihosting::hprintln;
use heapless::String;
// use stm32l4xx_hal::timer::*;

use stm32l4xx_hal::i2c::I2c;
use stm32l4xx_hal::rcc::RccExt;
use stm32l4xx_hal::{
    gpio::*,
    prelude::*,
    serial::{Config, Serial},
};

struct MCU {
    usart: Serial<
        stm32l4xx_hal::pac::USART2,
        (
            PA2<Alternate<AF7, Input<Floating>>>,
            PA3<Alternate<AF7, Input<Floating>>>,
        ),
    >,
    i2c: I2c<
        stm32l4xx_hal::pac::I2C1,
        (
            PB8<Alternate<AF4, Output<OpenDrain>>>,
            PB9<Alternate<AF4, Output<OpenDrain>>>,
        ),
    >,
}

fn setup_mcu() -> MCU {
    let device_periphs = stm32l4xx_hal::stm32::Peripherals::take().unwrap();
    let mut flash = device_periphs.FLASH.constrain();
    let mut reset_and_clock_control = device_periphs.RCC.constrain();

    let mut pwr = device_periphs
        .PWR
        .constrain(&mut reset_and_clock_control.apb1r1);

    let mut gpioa = device_periphs
        .GPIOA
        .split(&mut reset_and_clock_control.ahb2);
    let mut gpiob = device_periphs
        .GPIOB
        .split(&mut reset_and_clock_control.ahb2);

    let scl = gpiob
        .pb8
        .into_open_drain_output(&mut gpiob.moder, &mut gpiob.otyper)
        .into_af4(&mut gpiob.moder, &mut gpiob.afrh);

    let sda = gpiob
        .pb9
        .into_open_drain_output(&mut gpiob.moder, &mut gpiob.otyper)
        .into_af4(&mut gpiob.moder, &mut gpiob.afrh);

    let clocks = reset_and_clock_control
        .cfgr
        .freeze(&mut flash.acr, &mut pwr);

    let tx_pin = gpioa.pa2.into_af7(&mut gpioa.moder, &mut gpioa.afrl);
    let rx_pin = gpioa.pa3.into_af7(&mut gpioa.moder, &mut gpioa.afrl);
    let serial = Serial::usart2(
        device_periphs.USART2,
        (tx_pin, rx_pin),
        Config::default().baudrate(115200.bps()),
        clocks,
        &mut reset_and_clock_control.apb1r1,
    );

    let i2c = I2c::i2c1(
        device_periphs.I2C1,
        (scl, sda),
        100.khz(),
        clocks,
        &mut reset_and_clock_control.apb1r1,
    );
    MCU {
        usart: serial,
        i2c: i2c,
    }
}

#[entry]
fn main() -> ! {
    let mcu: MCU = setup_mcu();
    let mut i2c = mcu.i2c;
    let (mut tx, _rx) = mcu.usart.split();
    let tmp102_addr = 0x48;
    let mut msg: String<26> = String::new();

    loop {
        // Request temperature register (0x00)
        let mut buffer = [0; 2]; // Buffer to hold two bytes of temperature data
        i2c.write_read(tmp102_addr, &[0x00], &mut buffer).unwrap();

        // TMP102 returns two bytes: (MSB, LSB)
        let raw_temperature = ((buffer[0] as i16) << 4) | ((buffer[1] as i16) >> 4);

        // Convert raw data to Celsius
        let temperature_celsius = raw_temperature as f32 * 0.0625;

        core::writeln!(&mut msg, "Temperature is: {:02}\r", temperature_celsius).unwrap();
        for byte in msg.as_bytes().iter() {
            nb::block!(tx.write(*byte)).unwrap();
        }
        msg.clear();

        nb::block!(tx.flush()).unwrap();
    }
}
