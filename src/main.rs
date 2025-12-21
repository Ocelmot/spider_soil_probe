use std::{io, path::PathBuf};

use tokio::time::{self, Duration, sleep};

use spider_client::{
    
    link::{Relation, Role, SpiderId2048, message::{Message, UiElement, UiElementKind, UiMessage, UiPageManager, UiPath},}, SpiderClientBuilder, ClientChannel, ClientResponse,
};

use rppal::i2c::I2c;

const PROBE_ADDR: u16 = 0x36;

const TEMP_ADDR: u8 = 0x00;
const TEMP_SIZE: usize = 4;
const WATER_ADDR: u8 = 0x0f;
const WATER_SIZE: usize = 2;

struct State {
    pub test_page: UiPageManager,
}

impl State {
    async fn init(client: &mut ClientChannel) -> Self {
        let id = client.id().clone();
        let mut test_page = UiPageManager::new(id, "Probe");
        let mut root = test_page
            .get_element_mut(&UiPath::root())
            .expect("all pages have a root");
        root.set_kind(UiElementKind::Rows);
        root.append_child(UiElement::from_string("Temp is: "));
        root.append_child({
            let mut element = UiElement::from_string("-");
            element.set_id("temp");
            element
        });
        
        drop(root);

        test_page.get_changes(); // clear changes to synch, since we are going to send the whole page at first. This
                                 // Could instead set the initial elements with raw and then recalculate ids
        let msg = Message::Ui(UiMessage::SetPage(test_page.get_page().clone()));
        client.send(msg).await;

        Self {
            test_page,
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), io::Error> {
    println!("Hello, world!");



    let client_path = PathBuf::from("client_state.dat");

    let mut builder = SpiderClientBuilder::load_or_set(&client_path, |builder| {
        builder.enable_beacon(true);
    }).await.expect("Failed to load config");

    builder.try_use_keyfile("spider_keyfile.json").await;

    let mut client_channel = builder.start(true).await.expect("failed to start");


    let mut state = State::init(&mut client_channel).await;
    let mut i2c = I2c::new().unwrap();
    i2c.set_slave_address(PROBE_ADDR);

    let mut interval = time::interval(Duration::from_secs(10));
    loop {

        tokio::select!{
            // respond to base
            msg = client_channel.recv() => {
                match msg {
                    Ok(ClientResponse::Message(msg, _epoch)) => msg_handler(&mut client_channel, &mut state, msg).await,
                    Err(_e) => break, //  Client has failed in some way, exit
                    _ => {} // ignore other messages, since simple probe only returns data
                }
            },
            // Take probe reading
            x = interval.tick() => {
                let t = get_temp(&mut i2c).await;
                println!("Temp: {}", t);
                let mut element = state.test_page.get_by_id_mut("temp").expect("Page should have temp element");
                element.set_text(format!("{}C", t));

                drop(element);
                let changes = state.test_page.get_changes();
                let msg = Message::Ui(UiMessage::UpdateElements(changes));
                client_channel.send(msg).await;
            }
        }
    }

    Ok(())
}

// Do nothing, since this probe only sends messages
async fn msg_handler(client: &mut ClientChannel, state: &mut State, msg: Message) {
    match msg {
        Message::Ui(_msg) => {},
        Message::Dataset(_msg) => {},
        Message::Router(_msg) => {},
        Message::Group(_msg) => {},
        Message::Error(_msg) => {},
    }
}


async fn get_temp(i2c: &mut I2c) -> f32{
    let mut reg = [0x04];
    i2c.block_write(TEMP_ADDR, &mut reg).expect("write to succeed");
    sleep(Duration::from_millis(100)).await;
    let mut reg = [0u8; 4];
    let _data = i2c.block_read(TEMP_ADDR, &mut reg).expect("read to succeed");
    println!("bytes: {:?}", reg);
    let temp = i32::from_be_bytes(reg);
    let mut temp = temp as f32;
    temp = temp * 0.00001525878;
    temp
}