extern crate rumqtt;
extern crate serde;
extern crate serde_json;

#[macro_use]
extern crate serde_derive;

use rumqtt::{MqttCallback, MqttClient, MqttOptions, QoS};
use std::sync::{Arc, RwLock};
use std::thread;
use std::time::Duration;

trait Device {
    type Config: serde::Serialize;
    fn config(&self) -> Self::Config;
    fn config_topic(&self) -> String;
    fn id(&self) -> String;
    fn handle(&mut self, message: rumqtt::Message) -> Vec<rumqtt::Message>;
    fn state(&mut self) -> Vec<rumqtt::Message>;
}

#[derive(Debug)]
struct Light {
    pub name: String,
    pub id: String,
    pub brightness: u8,
}

impl Light {
    fn command_topic(&self) -> String {
        format!("home/{}/switch/set", self.id)
    }
    fn state_topic(&self) -> String {
        format!("home/{}/switch/status", self.id)
    }

    fn rgb_command_topic(&self) -> String {
        format!("home/{}/rgb/set", self.id)
    }
    fn rgb_state_topic(&self) -> String {
        format!("home/{}/rgb/status", self.id)
    }

    fn brightness_command_topic(&self) -> String {
        format!("home/{}/brightness/set", self.id)
    }
    fn brightness_state_topic(&self) -> String {
        format!("home/{}/brightness/status", self.id)
    }
}

impl Device for Light {
    type Config = LightConfig;
    fn config(&self) -> Self::Config {
        LightConfig {
            name: self.name.clone(),

            command_topic: self.command_topic(),
            state_topic: self.state_topic(),

            rgb_command_topic: self.rgb_command_topic(),
            rgb_state_topic: self.rgb_state_topic(),

            brightness_command_topic: self.brightness_command_topic(),
            brightness_state_topic: self.brightness_state_topic(),
        }
    }
    fn config_topic(&self) -> String {
        format!("homeassistant/light/{}/config", self.id)
    }
    fn id(&self) -> String {
        self.name.clone()
    }
    fn handle(&mut self, message: rumqtt::Message) -> Vec<rumqtt::Message> {
        let topic = message.topic.to_string();
        let payload = message.payload.to_vec();
        println!("topic --> {:?}", topic);
        let val = String::from_utf8(payload).expect("Could not parse string");
        println!("value --> {:?}", val);
        if topic == self.command_topic() {
            println!("command");
        } else if topic == self.rgb_command_topic() {
            println!("rgb");
        } else if topic == self.brightness_command_topic() {
            println!("brightness");
        } else {
            println!("other");
        };
        vec![]
    }
    fn state(&mut self) -> Vec<rumqtt::Message> {
        vec![]
    }
}

#[derive(Serialize, Debug)]
struct LightConfig {
    pub name: String,

    pub command_topic: String,
    pub state_topic: String,

    pub rgb_command_topic: String,
    pub rgb_state_topic: String,

    pub brightness_command_topic: String,
    pub brightness_state_topic: String,
}

fn main() {
    let light = Arc::new(RwLock::new(Light {
        name: "bedroom lights".to_string(),
        id: "bedroom/ceiling".to_string(),
        brightness: 0,
    }));

    let client_options = MqttOptions::new()
        .set_keep_alive(5)
        .set_reconnect(3)
        .set_client_id("rumqtt-docs")
        .set_broker("127.0.0.1:1883");

    let handler_light = light.clone();
    let cb = MqttCallback::new()
        .on_message(move |message| {
            println!("message --> {:?}", message);
            let updates = handler_light
                .write()
                .expect("Could not lock")
                .handle(message);
        })
        .on_publish(move |message| {
            println!("publish --> {:?}", message);
        });

    let client = Arc::new(RwLock::new(
        MqttClient::start(client_options, Some(cb)).expect("Could not start client"),
    ));

    client
        .write()
        .expect("Could not lock")
        .subscribe(vec![("home/#", QoS::Level0)])
        .expect("Could not subscribe");

    client
        .write()
        .expect("Could not lock")
        .publish("home/bedroom/ir", QoS::Level0, "ON".bytes().collect())
        .expect("Could not publish");

    {
        let client = client.clone();
        let interval = Duration::from_secs(5);
        thread::spawn(move || loop {
            println!("pushing config");
            client
                .write()
                .expect("Could not lock")
                .publish(
                    &light.read().expect("Could not lock").config_topic(),
                    QoS::Level0,
                    serde_json::to_vec(&light.read().expect("Could not lock").config())
                        .expect("Could not serialize to JSON"),
                )
                .expect("Could not publish");
            thread::sleep(interval);
        });
    }

    let forever = std::time::Duration::new(std::u64::MAX, 0);
    std::thread::sleep(forever);
}
