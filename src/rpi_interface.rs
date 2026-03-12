

use rppal::{i2c::I2c, gpio::{Gpio, InputPin, OutputPin, Event, Trigger, Level}};
use tokio::{time::{Duration, sleep}, sync::mpsc::{channel, Receiver}};

// I2C Constants
const PROBE_ADDR: u16 = 0x36;
const TEMP_ADDR: u8 = 0x00;
const TEMP_SIZE: usize = 4;
const WATER_ADDR: u8 = 0x0f;
const WATER_SIZE: usize = 2;

// GPIO Constants
pub const OUTPUT_SOLENOID_PIN: u8 = 25;
pub const INPUT_BUTTON_PIN: u8 = 23;
pub const OUTPUT_BUTTON_LED_PIN: u8 = 24;
pub const OUTPUT_PWR_LED_PIN: u8 = 16;

pub struct RpiInterface {
    i2c: I2c,
    gpio: Gpio,
    input_button: InputPin,
    button_output: OutputPin,
    solenoid_output: OutputPin,
    pwr_led_output: OutputPin,
    pin_events: Receiver<(u8, Trigger)>,
}


impl RpiInterface {
    pub fn new() -> Self {
        // init I2C
        let mut i2c = I2c::new().unwrap();
        i2c.set_slave_address(PROBE_ADDR);

        // Init GPIO
        let gpio = Gpio::new().expect("Failed to attach to GPIO pins");
        let (pin_events_input, pin_events) = channel::<(u8, Trigger)>(10);

        // Input Button
        let mut input_button = gpio.get(INPUT_BUTTON_PIN).expect("input button pin not available").into_input_pullup();
        let pin_button_input = pin_events_input.clone();
        input_button.set_async_interrupt(Trigger::Both, Some(Duration::from_millis(25)), move |event: Event| {
            pin_button_input.blocking_send((INPUT_BUTTON_PIN, event.trigger));
        });

        let button_output = gpio.get(OUTPUT_BUTTON_LED_PIN).expect("Button output pin not available").into_output();
        let solenoid_output = gpio.get(OUTPUT_SOLENOID_PIN).expect("Button output pin not available").into_output();
        let pwr_led_output = gpio.get(OUTPUT_PWR_LED_PIN).expect("Button output pin not available").into_output();

        Self {
            i2c,
            gpio,

            input_button,
            button_output,
            solenoid_output,
            pwr_led_output,

            pin_events,
        }
    }

    pub async fn get_temp(&mut self) -> f32 {
        let mut reg = [0x04];
        self.i2c.block_write(TEMP_ADDR, &mut reg).expect("write to succeed");
        sleep(Duration::from_millis(10)).await;
        let mut reg = [0u8; TEMP_SIZE];
        let _data = self.i2c.block_read(TEMP_ADDR, &mut reg).expect("read to succeed");
        println!("bytes: {:?}", reg);
        let temp = i32::from_be_bytes(reg);
        let mut temp = temp as f32;
        temp = temp * 0.00001525878;
        temp
    }

    pub async fn get_water(&mut self) -> u16 {
        let mut reg = [0x10];
        self.i2c.block_write(WATER_ADDR, &mut reg).expect("write to succeed");
        sleep(Duration::from_millis(10)).await;
        let mut reg = [0u8; WATER_SIZE];
        let _data = self.i2c.block_read(WATER_ADDR, &mut reg).expect("read to succeed");
        println!("bytes: {:?}", reg);
        let water = u16::from_be_bytes(reg);
        water
    }

    pub async fn get_pin_event(&mut self) -> Option<(u8, Trigger)> {
        self.pin_events.recv().await
    }

    pub fn get_button_led_level(&mut self) -> bool {
        self.button_output.is_set_high()
    }

    pub fn set_button_led_level<V: Into<Level>>(&mut self, level: V) {
        self.button_output.write(level.into());
    }

    pub fn get_pwr_led_level(&mut self) -> bool {
        self.pwr_led_output.is_set_high()
    }

    pub fn set_pwr_led_level<V: Into<Level>>(&mut self, level: V) {
        self.pwr_led_output.write(level.into());
    }

    pub fn get_solenoid_level(&mut self) -> bool {
        self.solenoid_output.is_set_high()
    }

    pub fn set_solenoid_level<V: Into<Level>>(&mut self, level: V) {
        self.solenoid_output.write(level.into());
    }
}





