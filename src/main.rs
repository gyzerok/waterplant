#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]

use embassy_executor::Executor;
use embassy_time::{Delay, Duration, Timer};
use embedded_svc::wifi::{ClientConfiguration, Configuration, Wifi};
use esp_backtrace as _;
use esp_println::println;
use esp_wifi::{
    self,
    wifi::{WifiController, WifiDevice, WifiEvent, WifiMode, WifiState},
};
use hal::{
    adc::{AdcConfig, AdcPin, Attenuation, ADC, ADC1},
    clock::{ClockControl, CpuClock},
    embassy,
    gpio::{Analog, GpioPin},
    i2c::I2C,
    interrupt,
    peripherals::{Interrupt, Peripherals, I2C0},
    prelude::*,
    systimer::SystemTimer,
    timer::TimerGroup,
    Rng, Rtc, IO,
};
use static_cell::StaticCell;
use waterplant::lcd_i2c::Lcd;

static EXECUTOR: StaticCell<Executor> = StaticCell::new();

// const SSID: &str = env!("WIFI_SSID");
// const PASSWORD: &str = env!("WIFI_PASS");

#[entry]
fn main() -> ! {
    esp_println::println!("Init!");
    let peripherals = Peripherals::take();
    let mut system = peripherals.SYSTEM.split();
    // let clocks = ClockControl::boot_defaults(system.clock_control).freeze();
    let clocks = ClockControl::configure(system.clock_control, CpuClock::Clock160MHz).freeze();

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

    embassy::init(&clocks, timer_group0.timer0);

    let io = IO::new(peripherals.GPIO, peripherals.IO_MUX);

    // ADC FOR SOIL MOISTURE SENSOR (SMS) SETUP

    let mut adc1_config = AdcConfig::new();
    let sms_pin = adc1_config.enable_pin(io.pins.gpio1.into_analog(), Attenuation::Attenuation11dB);
    let analog = peripherals.APB_SARADC.split();
    let sms = ADC::adc(
        &mut system.peripheral_clock_control,
        analog.adc1,
        adc1_config,
    )
    .unwrap();

    // I2C FOR LCD SETUP

    let i2c0 = I2C::new(
        peripherals.I2C0,
        io.pins.gpio4,
        io.pins.gpio5,
        400u32.kHz(),
        &mut system.peripheral_clock_control,
        &clocks,
    );

    interrupt::enable(Interrupt::I2C_EXT0, interrupt::Priority::Priority1).unwrap();

    // WIFI SETUP

    let init = esp_wifi::initialize(
        esp_wifi::EspWifiInitFor::Wifi,
        SystemTimer::new(peripherals.SYSTIMER).alarm0,
        Rng::new(peripherals.RNG),
        system.radio_clock_control,
        &clocks,
    )
    .unwrap();

    let (wifi_dev, _) = peripherals.RADIO.split();
    let (wifi_iface, controller) =
        esp_wifi::wifi::new_with_mode(&init, wifi_dev, WifiMode::Sta).unwrap();

    // STARTING TASKS

    let executor = EXECUTOR.init(Executor::new());
    executor.run(|spawner| {
        // spawner.spawn(run_lcd(i2c0)).ok();
        spawner.spawn(run_moisure_sensor(sms, sms_pin)).ok();
        // spawner.spawn(connect_wifi(controller)).ok();
    });
}

#[embassy_executor::task]
async fn run_lcd(i2c: I2C<'static, I2C0>) {
    let mut lcd1602 = Lcd::new(i2c, Delay).with_2rows().enable().await.unwrap();

    lcd1602.write_str("Hello, world!").await.unwrap();
}

#[embassy_executor::task]
async fn run_pwm() {
    loop {
        esp_println::println!("Hello world from embassy using esp-hal-async!");
        Timer::after(Duration::from_millis(1_000)).await;
    }
}

#[embassy_executor::task]
async fn run_moisure_sensor(
    mut sms: ADC<'static, ADC1>,
    mut sms_pin: AdcPin<GpioPin<Analog, 1>, ADC1>,
) {
    loop {
        let sample: u16 = nb::block!(sms.read(&mut sms_pin)).unwrap();
        println!("Moisture level: {:?}", sample);
        Timer::after(Duration::from_millis(2000)).await;
    }
}

// #[embassy_executor::task]
// async fn connect_wifi(mut controller: WifiController<'static>) {
//     println!("start connection task");
//     println!("Device capabilities: {:?}", controller.get_capabilities());
//     loop {
//         match esp_wifi::wifi::get_wifi_state() {
//             WifiState::StaConnected => {
//                 // wait until we're no longer connected
//                 controller.wait_for_event(WifiEvent::StaDisconnected).await;
//                 Timer::after(Duration::from_millis(5000)).await
//             }
//             _ => {}
//         }

//         if !matches!(controller.is_started(), Ok(true)) {
//             let client_config = Configuration::Client(ClientConfiguration {
//                 ssid: SSID.into(),
//                 password: PASSWORD.into(),
//                 ..Default::default()
//             });
//             controller.set_configuration(&client_config).unwrap();
//             println!("Starting wifi");
//             controller.start().await.unwrap();
//             println!("Wifi started!");
//         }
//         println!("About to connect...");

//         match controller.connect().await {
//             Ok(_) => println!("Wifi connected!"),
//             Err(e) => {
//                 println!("Failed to connect to wifi: {e:?}");
//                 Timer::after(Duration::from_millis(5000)).await
//             }
//         }
//     }
// }
