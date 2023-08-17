#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]

use embassy_executor::Executor;
use embassy_time::{Duration, Timer};
use esp_backtrace as _;
use esp_println::println;
use hal::{
    clock::ClockControl, embassy, peripherals::Peripherals, prelude::*, timer::TimerGroup, Rtc,
};
use static_cell::StaticCell;

static EXECUTOR: StaticCell<Executor> = StaticCell::new();

#[entry]
fn main() -> ! {
    esp_println::println!("Init!");
    let peripherals = Peripherals::take();
    let mut system = peripherals.SYSTEM.split();
    let clocks = ClockControl::boot_defaults(system.clock_control).freeze();

    let mut rtc = Rtc::new(peripherals.RTC_CNTL);
    let timer_group0 = TimerGroup::new(
        peripherals.TIMG0,
        &clocks,
        &mut system.peripheral_clock_control,
    );
    let mut wdt0 = timer_group0.wdt;
    let timer_group1 = TimerGroup::new(
        peripherals.TIMG1,
        &clocks,
        &mut system.peripheral_clock_control,
    );
    let mut wdt1 = timer_group1.wdt;

    // Disable watchdog timers
    rtc.swd.disable();
    rtc.rwdt.disable();
    wdt0.disable();
    wdt1.disable();

    embassy::init(
        &clocks,
        hal::systimer::SystemTimer::new(peripherals.SYSTIMER),
    );

    let executor = EXECUTOR.init(Executor::new());
    executor.run(|spawner| {
        spawner.spawn(run1()).ok();
        spawner.spawn(run2()).ok();
    });
}

#[embassy_executor::task]
async fn run1() {
    loop {
        esp_println::println!("Hello world from embassy using esp-hal-async!");
        Timer::after(Duration::from_millis(1_000)).await;
    }
}

#[embassy_executor::task]
async fn run2() {
    loop {
        esp_println::println!("Bing!");
        Timer::after(Duration::from_millis(5_000)).await;
    }
}

// #![no_std]
// #![no_main]
// #![feature(type_alias_impl_trait)]

// use core::fmt::Write;
// use core::write;

// use embassy_executor::Spawner;
// use embassy_rp::bind_interrupts;
// use embassy_rp::i2c::{self, Config, InterruptHandler};
// use embassy_rp::peripherals::I2C1;
// use embassy_time::{Delay, Duration, Timer};
// use fntest::lcd_i2c::Lcd;
// use heapless::String;
// use {defmt, defmt_rtt as _, panic_probe as _};

// bind_interrupts!(struct Irqs {
//     I2C1_IRQ => InterruptHandler<I2C1>;
// });

// #[embassy_executor::main]
// async fn main(_spawner: Spawner) {
//     let p = embassy_rp::init(Default::default());

//     let sda = p.PIN_14;
//     let scl = p.PIN_15;

//     let i2c = i2c::I2c::new_async(p.I2C1, scl, sda, Irqs, Config::default());
//     let mut lcd1602 = Lcd::new(i2c, Delay).with_2rows().enable().await.unwrap();

//     lcd1602.write_str("Hello, world!").await.unwrap();

//     lcd1602.move_cursor_to(1, 0).await.unwrap();

//     let mut output: String<17> = String::new();
//     write!(&mut output, "  {} {} {}", 12, 21.05, true).unwrap();
//     lcd1602.write_str(&output).await.unwrap();

//     let dick = lcd1602
//         .register_custom_char(0, &[0x04, 0x0A, 0x0E, 0x0A, 0x0A, 0x0A, 0x15, 0x1B])
//         .await
//         .unwrap();
//     lcd1602.move_cursor_to(1, 0).await.unwrap();
//     lcd1602.write_u8(dick).await.unwrap();

//     Timer::after(Duration::from_secs(10)).await;
// }
