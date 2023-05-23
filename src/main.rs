use std::{io, path::PathBuf};

use tokio::time::{self, Duration, sleep};

use spider_client::{
    message::{Message, UiElement, UiElementKind, UiMessage, UiPageManager, UiPath},
    AddressStrategy, Relation, Role, SpiderClient, SpiderId2048,
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
    async fn init(client: &mut SpiderClient) -> Self {
        let id = client.self_relation().id;
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
    let mut client = if client_path.exists() {
        SpiderClient::from_file(&client_path)
    } else {
        let mut client = SpiderClient::new();
        client.set_state_path(&client_path);
        client.add_strat(AddressStrategy::Addr(String::from("192.168.0.10:1930")));
        client.save();
        client
    };

    if !client.has_host_relation() {
        let path = PathBuf::from("spider_keyfile.json");

        let data = match std::fs::read_to_string(&path) {
            Ok(str) => str,
            Err(_) => String::from("[]"),
        };
        let id: SpiderId2048 = serde_json::from_str(&data).expect("Failed to deserialize spiderid");
        let host = Relation {
            id,
            role: Role::Peer,
        };
        client.set_host_relation(host);
        client.save();
    }

    client.connect().await;
    let mut state = State::init(&mut client).await;
    let mut i2c = I2c::new().unwrap();
    i2c.set_slave_address(PROBE_ADDR);

    let mut interval = time::interval(Duration::from_secs(10));
    loop {

        tokio::select!{
            // respond to base
            msg = client.recv() => {
                match msg {
                    Some(msg) => msg_handler(&mut client, &mut state, msg).await,
                    None => break, //  done! (Maybe retry connection)
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
                client.send(msg).await;
            }
        }
    }

    Ok(())
}

// Do nothing, since this probe only sends messages
async fn msg_handler(client: &mut SpiderClient, state: &mut State, msg: Message) {
    match msg {
        Message::Peripheral(_) => {}
        Message::Ui(_) => {},
        Message::Dataset(_) => {}
        Message::Event(_) => {}
    }
}


async fn get_temp(i2c: &mut I2c) -> f32{
    let mut reg = [0x04];
    i2c.block_write(TEMP_ADDR, &mut reg).expect("write to succeed");
    sleep(Duration::from_millis(100)).await;
    let mut reg = [0u8; 4];
    let data = i2c.block_read(TEMP_ADDR, &mut reg).expect("read to succeed");
    println!("bytes: {:?}", reg);
    let temp = i32::from_be_bytes(reg);
    let mut temp = temp as f32;
    temp = temp * 0.00001525878;
    temp
}