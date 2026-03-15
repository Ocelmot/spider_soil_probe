
use crate::rpi_interface::RpiInterface;
use crate::ui::init_ui;

use std::time::{Duration, Instant};
use spider_client::{link::message::{Message, UiMessage, RouterMessage, UiPageManager, DatasetMessage, DatasetPath, DatasetData}, ClientChannel};
use rppal::gpio::Trigger;
use chrono::{Local, NaiveTime};
use std::collections::HashMap;

pub const PWR_LED_CONFIG: &'static str = "pwr_led";
pub const BUTTON_LED_CONFIG: &'static str = "button_led";
pub const TEMP_UNIT_CONFIG_C: &'static str = "temp_unit";

pub const SCHED_START: &'static str = "sched_start";
pub const SCHED_DURATION: &'static str = "sched_duration";
pub const SCHED_MAX_WATER: &'static str = "sched_max_water";

pub struct State {
    dataset_path: DatasetPath,
    client_sender: ClientChannel,
    rpi_interface: RpiInterface,
    ui_page: UiPageManager,
    remaining_time: Duration,
    last_press: Instant,
    config: HashMap<String, DatasetData>,
}

impl State {
    pub async fn init(client_sender: ClientChannel) -> Self {
        // request dataset item
        let path = DatasetPath::new_private(vec!["state".into()]);
        client_sender.send(Message::Dataset(DatasetMessage::Subscribe{path: path.clone()})).await;
        
        let rpi_interface = RpiInterface::new();
        let ui_page = init_ui( client_sender.id().clone());

        let mut ret = Self {
            dataset_path: path,
            client_sender,
            rpi_interface,
            ui_page,
            remaining_time: Duration::ZERO,
            last_press: Instant::now(),
            config: HashMap::new(),
        };
        // use default until the saved dataset comes back from the base.
        let map = HashMap::from([
            (String::from(PWR_LED_CONFIG), DatasetData::Byte(1)),
            (String::from(BUTTON_LED_CONFIG), DatasetData::Byte(1)),
            (String::from(TEMP_UNIT_CONFIG_C), DatasetData::Byte(1)),

            (String::from(SCHED_START), DatasetData::String(String::from("00:00"))),
            (String::from(SCHED_DURATION), DatasetData::Int(0)),
            (String::from(SCHED_MAX_WATER), DatasetData::Int(0)),
        ]);
        ret.set_config(map).await;
        ret
    }

    pub async fn update_page_temp(&mut self) {
        let c = self.rpi_interface.get_temp().await;
        println!("Temp: {}°C", c);
        let is_c = self.is_units_c();
        let mut element = self.ui_page.get_by_id_mut("temp").expect("Page should have temp element");
        if is_c {
            element.set_text(format!("{}°C", c));
        }else{
            let f = (c * 1.8) + 32.0; 
            element.set_text(format!("{}°F", f));
        }
        
    }

    pub async fn update_page_water(&mut self) {
        let t = self.rpi_interface.get_water().await;
        println!("Water: {}", t);
        let mut element = self.ui_page.get_by_id_mut("water").expect("Page should have water element");
        element.set_text(format!("{}", t));
    }

    pub fn update_page_timer(&mut self) {
        let mut element = self.ui_page.get_by_id_mut("remaining_time").expect("Page should have remaining_time element");
        let secs = self.remaining_time.as_secs();
        element.set_text(format!("{}:{:02}", secs/60, secs%60));
    }

    pub fn update_page_clock_time(&mut self) {
        let mut element = self.ui_page.get_by_id_mut("clock_time").expect("Page should have clock_time element");
        let time = Local::now();
        let formatted = format!("{}", time.format("%-I:%M %p %Z"));
        element.set_text(formatted);
    }

    pub async fn update_ui(&mut self) {
        let changes = self.ui_page.get_changes();
        //println!("Change set: {:?}", changes);
        if changes.is_empty() {
            return;
        }
        let msg = Message::Ui(UiMessage::UpdateElements(changes));
        self.client_sender.send(msg).await;
    }

    pub async fn on_connect(&mut self) {
        // Send the name of the peripheral
        let msg = RouterMessage::SetIdentityProperty("name".into(), "Sluice".into());
        let msg = Message::Router(msg);
        self.client_sender.send(msg).await;

        // Send the current UI page
        self.ui_page.get_changes(); // clear changes to synch, since we are going to send the whole page at first. This
                                    // Could instead set the initial elements with raw and then recalculate ids
        let msg = Message::Ui(UiMessage::SetPage(self.ui_page.get_page().clone()));
        self.client_sender.send(msg).await;
    }

    pub fn increment_time(&mut self, duration: Duration) {
        if duration < Duration::from_secs(1) {
            return;
        }
        
        if self.remaining_time.is_zero() {
            self.rpi_interface.set_solenoid_level(true);
            if self.is_enable_button_led() {
                self.rpi_interface.set_button_led_level(true);
            }
        }

        self.remaining_time += duration;
    }

    pub fn decrement_time(&mut self, duration: Duration){
        if duration.is_zero() {
            return;
        }
        self.remaining_time = self.remaining_time.saturating_sub(duration);
        if self.remaining_time.is_zero() {
            self.rpi_interface.set_solenoid_level(false);
            self.rpi_interface.set_button_led_level(false);
        }
    }

    pub fn clear_time(&mut self){
        self.remaining_time = Duration::ZERO;        
        self.rpi_interface.set_solenoid_level(false);
        self.rpi_interface.set_button_led_level(false);
    }

    pub fn start_press(&mut self) {
        self.last_press = Instant::now();
    }

    pub fn time_since_press(&self) -> Duration {
        Instant::now() - self.last_press
    }

    pub async fn get_pin_event(&mut self) -> Option<(u8, Trigger)> {
        self.rpi_interface.get_pin_event().await
    }

    pub async fn process_schedule(&mut self, duration: Duration) -> Option<()>{
        // dont start if running.
        if !self.remaining_time.is_zero(){
            return None;
        }

        let duration = duration + duration / 2;
        // Get current time
        let current_time = Local::now().time();

        // Check time
        let DatasetData::String(set_start_time) = self.config.get(&String::from(SCHED_START))? else {return None};
        let set_time = NaiveTime::parse_from_str(&set_start_time, "%I:%M %P").ok()?;
        if set_time > current_time || current_time > set_time + duration {
            return None;
        }

        // Check moisture
        let DatasetData::Int(set_moisture) = self.config.get(&String::from(SCHED_MAX_WATER))? else {return None};
        if *set_moisture != 0 && *set_moisture < (self.rpi_interface.get_water().await).into() {
            return None;
        }

        // Add time
        let DatasetData::Int(duration_mins) = self.config.get(&String::from(SCHED_DURATION))? else {return None};
        let duration = Duration::from_secs((*duration_mins).try_into().unwrap_or(0u64) * 60);
        self.increment_time(duration);
        
        Some(())
    }


    // Config functions

    pub async fn set_config(&mut self, config: HashMap::<String, DatasetData>) {
        self.config = config;
        if let Some(DatasetData::Byte(state)) = self.config.get(&String::from(PWR_LED_CONFIG)) {
            self.set_enable_pwr_led(*state != 0).await;
        }
        if let Some(DatasetData::Byte(state)) = self.config.get(&String::from(BUTTON_LED_CONFIG)) {
            self.set_enable_button_led(*state != 0).await;
        }
        if let Some(DatasetData::Byte(state)) = self.config.get(&String::from(TEMP_UNIT_CONFIG_C)) {
            self.set_units_c(*state != 0).await;
        }

        if let Some(DatasetData::String(text)) = self.config.get(&String::from(SCHED_START)) {
            if let Ok(time) = NaiveTime::parse_from_str(&text, "%I:%M %P") {
                self.set_sched_time(time);
            }
        }
        if let Some(DatasetData::Int(duration)) = self.config.get(&String::from(SCHED_DURATION)) {
            self.set_sched_duration(*duration as u32);
        }
        if let Some(DatasetData::Int(water)) = self.config.get(&String::from(SCHED_MAX_WATER)) {
            self.set_sched_max_water(*water as u32);
        }
    }
    

    // Schedule Config

    pub fn set_sched_time(&mut self, time: NaiveTime) {
        let time_str = time.format("%I:%M %P").to_string();

        // update UI
        let mut element = self.ui_page.get_by_id_mut("start_time").expect("Page should have start time config element");
        element.set_text(time_str.clone());

        // update config
        self.config.insert(String::from(SCHED_START), DatasetData::String(time_str));
    }

    pub fn set_sched_duration(&mut self, duration: u32) {
        let mut element = self.ui_page.get_by_id_mut("duration").expect("Page should have duration config element");
        element.set_text(format!("{} mins", duration));

        self.config.insert(String::from(SCHED_DURATION), DatasetData::Int(duration.try_into().unwrap_or(0)));
    }

    pub fn set_sched_max_water(&mut self, water: u32) {
        let mut element = self.ui_page.get_by_id_mut("max_water").expect("Page should have max water config element");
        element.set_text(format!("{}", water));

        self.config.insert(String::from(SCHED_MAX_WATER), DatasetData::Int(water.try_into().unwrap_or(0)));
    }



    // Settings Config

    pub fn is_units_c(&self) -> bool{
        if let Some(DatasetData::Byte(state)) = self.config.get(&String::from(TEMP_UNIT_CONFIG_C)) {
            *state != 0
        }else{
            false
        }
    }

    pub async fn set_units_c(&mut self, state: bool) {
        // update config
        self.config.insert(String::from(TEMP_UNIT_CONFIG_C), DatasetData::Byte(if state {1} else {0}));

        // update UI
        if state{
            let mut element = self.ui_page.get_by_id_mut("toggle_temp_unit").expect("Page should have temp unit config element");
            element.set_text(String::from("°C"));
        }else{
            let mut element = self.ui_page.get_by_id_mut("toggle_temp_unit").expect("Page should have temp unit config element");
            element.set_text(String::from("°F"));
        }
        self.update_page_temp().await;
        self.update_ui().await;
    }

    pub fn is_enable_pwr_led(&self) -> bool{
        if let Some(DatasetData::Byte(state)) = self.config.get(&String::from(PWR_LED_CONFIG)) {
            *state != 0
        }else{
            false
        }
    }

    pub async fn set_enable_pwr_led(&mut self, state: bool) {
        // update hardware
        self.rpi_interface.set_pwr_led_level(state);

        // update UI
        if state{
            let mut element = self.ui_page.get_by_id_mut("toggle_pwr_led").expect("Page should have power led config element");
            element.set_text(String::from("Disable"));
        }else{
            let mut element = self.ui_page.get_by_id_mut("toggle_pwr_led").expect("Page should have power led config element");
            element.set_text(String::from("Enable"));
        }
        self.update_ui().await;

        // update config
        self.config.insert(String::from(PWR_LED_CONFIG), DatasetData::Byte(if state {1} else {0}));
    }

    pub fn is_enable_button_led(&self) -> bool{
        if let Some(DatasetData::Byte(state)) = self.config.get(&String::from(BUTTON_LED_CONFIG)) {
            *state != 0
        }else{
            false
        }
    }

    pub async fn set_enable_button_led(&mut self, state: bool) {
        // update hardware
        if state{
            if !self.remaining_time.is_zero() {
                self.rpi_interface.set_button_led_level(true);
            }
        }else{
            self.rpi_interface.set_button_led_level(false);
        }

        // update UI
        if state{
            let mut element = self.ui_page.get_by_id_mut("toggle_button_led").expect("Page should have button led config element");
            element.set_text(String::from("Disable"));
        }else{
            let mut element = self.ui_page.get_by_id_mut("toggle_button_led").expect("Page should have button led config element");
            element.set_text(String::from("Enable"));
        }
        self.update_ui().await;

        // update config
        self.config.insert(String::from(BUTTON_LED_CONFIG), DatasetData::Byte(if state {1} else {0}));
    }

    pub async fn save_config(&mut self) {
        let msg = Message::Dataset(DatasetMessage::SetElement{path: self.dataset_path.clone(), data: DatasetData::Map(self.config.clone()), id: 0});
        self.client_sender.send(msg).await;
    }
}