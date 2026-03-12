

mod rpi_interface;
use rpi_interface::INPUT_BUTTON_PIN;

mod state;
use state::State;

mod ui;

use std::{io, path::PathBuf, time::Duration};
use tokio::time::interval;
use spider_client::{SpiderClientBuilder, ClientResponse, link::message::{Message, UiMessage, UiInput, DatasetPath, DatasetMessage, DatasetData}};
use rppal::gpio::Trigger;

const TICK_INTERVAL: Duration = Duration::from_secs(10);
const LONG_PRESS_THRESHOLD: Duration = Duration::from_secs(4);

#[tokio::main]
async fn main() -> Result<(), io::Error> {
    println!("Hello, world!");

    let client_path = PathBuf::from("client_state.dat");

    let mut builder = SpiderClientBuilder::load_or_set(&client_path, |builder| {
        builder.enable_beacon(true);
    }).await.expect("Failed to load config");

    builder.try_use_keyfile("spider_keyfile.json").await;

    let mut client_channel = builder.start(true).await.expect("failed to start");
    let client_sender = client_channel.clone();

    let mut state = State::init(client_sender).await;
    

    let mut interval = interval(Duration::from_secs(1));
    loop {
        tokio::select!{
            // respond to base
            msg = client_channel.recv() => {
                println!("Got message: {:?}", msg);
                match msg {
                    Ok(ClientResponse::Connected(_epoch)) => {
                        // println!("Connected!");
                        state.on_connect().await;
                    }
                    Ok(ClientResponse::Message(Message::Ui(UiMessage::Input(element_id, dataset_ids, change)), _epoch)) => handle_input(&mut state, element_id, dataset_ids, change).await,
                    Ok(ClientResponse::Message(Message::Dataset(DatasetMessage::Dataset{path, data}), _epoch)) => handle_dataset(&mut state, path, data).await,
                    Ok(_response) => {
                        // println!("got response: {:?}", response);
                    }
                    Err(_e) => break, //  Client has failed in some way, exit
                }
            },
            // Take probe reading
            _ = interval.tick() => {
                println!("period: {:?}", interval.period());
                state.update_page_clock_time();
                state.update_page_temp().await;
                state.update_page_water().await;
                state.decrement_time(interval.period());
                state.update_page_timer();
                state.update_ui().await;
            },
            // Respond when the pins change their state
            pin_event = state.get_pin_event() => {
                let (pin, trigger) = pin_event.expect("Event channel closed");
                println!("pin: {:?}, trigger: {:?}", pin, trigger);
                if pin == INPUT_BUTTON_PIN  && trigger == Trigger::FallingEdge{
                    state.start_press();
                }
                if pin == INPUT_BUTTON_PIN  && trigger == Trigger::RisingEdge{
                    // if timer was over the hold limit
                    if state.time_since_press() > LONG_PRESS_THRESHOLD {
                        println!("Resetting time");
                        state.clear_time();
                        state.update_page_timer();
                        state.update_ui().await;
                    }else{
                        println!("Incrementing time");
                        state.increment_time(Duration::from_secs(10));
                        state.update_page_timer();
                        state.update_ui().await;
                    }
                }
            }
        }
    }

    Ok(())
}

async fn handle_input(state: &mut State, element_id: String, _dataset_ids: Vec<usize>, _change: UiInput){
    match element_id.as_str() {
        "inc_5_min" => {
            state.increment_time(Duration::from_mins(5));
            state.update_page_timer();
            state.update_ui().await;
        }
        "inc_10_min" => {
            state.increment_time(Duration::from_mins(10));
            state.update_page_timer();
            state.update_ui().await;
        }
        "inc_1_day" => {
            state.increment_time(Duration::from_hours(24));
            state.update_page_timer();
            state.update_ui().await;
        }
        "clear_timer" => {
            state.clear_time();
            state.update_page_timer();
            state.update_ui().await;
        }

        "toggle_pwr_led" => {
            let mut pwr_led = state.is_enable_pwr_led();
            pwr_led = !pwr_led;
            println!("Toggling power LED: new_value: {}", pwr_led);
            state.set_enable_pwr_led(pwr_led).await;
            state.save_config().await;
        }

        "toggle_temp_unit" => {
            let mut is_c = state.is_units_c();
            is_c = !is_c;
            println!("Toggling unit: new_value: {}", is_c);
            state.set_units_c(is_c).await;
            state.save_config().await;
        }

        "toggle_button_led" => {
            let mut pwr_led = state.is_enable_button_led();
            pwr_led = !pwr_led;
            println!("Toggling button LED: new_value: {}", pwr_led);
            state.set_enable_button_led(pwr_led).await;
            state.save_config().await;
        }

        _ => {} // Unknown input?
    }
}

async fn handle_dataset(state: &mut State, _path: DatasetPath, data: Vec<DatasetData>){
    if let Some(DatasetData::Map(map)) = data.into_iter().nth(0){
        // use saved config
        state.set_config(map).await;
    }
}