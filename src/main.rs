extern crate getopts;
extern crate mote;
extern crate palette;
extern crate rand;
extern crate rgb;
extern crate rumqtt;
extern crate serde;
extern crate serde_json;

#[macro_use]
extern crate serde_derive;

use palette::blend::Blend;
use palette::Mix;
use rand::distributions::Distribution;
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

const BACKGROUND: palette::rgb::LinSrgb = palette::rgb::LinSrgb {
    red: 0.0,
    green: 0.0,
    blue: 0.0,
    standard: std::marker::PhantomData,
};

#[derive(Debug)]
struct Particle {
    creation_time: u64,
    color: palette::rgb::LinSrgb,
}

#[derive(Debug)]
struct Segment {
    particles: Vec<Particle>,
    pixels: Vec<palette::rgb::LinSrgb>,
}

impl Default for Segment {
    fn default() -> Self {
        Segment {
            particles: vec![],
            pixels: [BACKGROUND; mote::PIXELS_PER_CHANNEL as usize].to_vec(),
        }
    }
}

#[derive(Debug)]
struct Light {
    pub name: String,
    pub id: String,
    pub brightness: u8,

    lambda: u8,
    decay: u8,
    rate: u8,

    segments: Vec<Segment>,
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

    fn lambda_command_topic(&self) -> String {
        format!("home/{}/lambda/set", self.id)
    }
    fn decay_command_topic(&self) -> String {
        format!("home/{}/decay/set", self.id)
    }
    fn rate_command_topic(&self) -> String {
        format!("home/{}/rate/set", self.id)
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
        } else if topic == self.lambda_command_topic() {
            println!("lambda");
            self.lambda = val.parse().unwrap();
        } else if topic == self.decay_command_topic() {
            println!("decay");
            self.decay = val.parse().unwrap();
        } else if topic == self.rate_command_topic() {
            println!("rate");
            self.rate = val.parse().unwrap();
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
    let mut opts = getopts::Options::new();
    opts.optopt("", "mqtt_broker_address", "", "");
    opts.optopt("", "mqtt_broker_username", "", "");
    opts.optopt("", "mqtt_broker_password", "", "");
    let matches = opts.parse(std::env::args()).unwrap();

    let light = Arc::new(RwLock::new(Light {
        name: "bedroom lights".to_string(),
        id: "bedroom/ceiling".to_string(),
        brightness: 0,

        lambda: 128,
        decay: 128,
        rate: 128,

        segments: vec![
            Segment::default(),
            Segment::default(),
            Segment::default(),
            Segment::default(),
        ],
    }));

    let mut client_options = MqttOptions::new()
        .set_keep_alive(5)
        .set_reconnect(3)
        .set_client_id("rumqtt-docs")
        .set_broker("127.0.0.1:1883");

    if let Some(mqtt_address) = matches.opt_str("mqtt_broker_address") {
        client_options = client_options.set_broker(&mqtt_address);
    }
    if let Some(mqtt_username) = matches.opt_str("mqtt_broker_username") {
        client_options = client_options.set_user_name(&mqtt_username);
    }
    if let Some(mqtt_password) = matches.opt_str("mqtt_broker_password") {
        client_options = client_options.set_password(&mqtt_password);
    }

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
        /*
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
        */

        let background_light = light.clone();
        thread::spawn(move || {
            println!("start");
            let mut mote = mote::Mote::new("/dev/ttyACM0", true);
            mote.clear();

            let mut rng = rand::thread_rng();

            let mut n = 0u64;
            loop {
                let mut interval = std::time::Duration::from_millis(50);
                {
                    let mut light = background_light.write().unwrap();
                    interval = std::time::Duration::from_millis(1000 / light.rate as u64);

                    let decay = 1.0 * light.decay as f32 / 255.0;

                    let dist = rand::distributions::Poisson::new(2.0 * light.lambda as f64);
                    for i in 0..light.segments.len() {
                        let mut segment = &mut light.segments[i];
                        if dist.sample(&mut rng) > 1 {
                            segment.particles.push(Particle {
                                creation_time: n,
                                color: random_color(),
                            });
                        }
                        let mask = make_mask(&segment.particles, n);
                        for i in 0..mote::PIXELS_PER_CHANNEL as usize {
                            segment.pixels[i] = segment.pixels[i].screen(mask[i]);
                            segment.pixels[i] = segment.pixels[i].mix(&BACKGROUND, decay);
                        }
                    }

                    let pixels = light
                        .segments
                        .iter()
                        .flat_map(|x| x.pixels.clone())
                        .collect::<Vec<_>>();
                    mote.write(&to_array(&pixels.iter().map(to_rgb).collect::<Vec<_>>()));
                }
                std::thread::sleep(std::time::Duration::from_millis(50));
                n += 1;
            }
        });
    }

    let forever = std::time::Duration::new(std::u64::MAX, 0);
    std::thread::sleep(forever);
}

fn to_array(pixels: &[rgb::RGB8]) -> [rgb::RGB8; mote::TOTAL_PIXELS] {
    let mut out = [to_rgb(&BACKGROUND); mote::TOTAL_PIXELS];
    for i in 0..pixels.len() {
        out[i] = pixels[i];
    }
    out
}

fn make_mask(particles: &Vec<Particle>, n: u64) -> [palette::rgb::LinSrgb; mote::TOTAL_PIXELS] {
    // Speed in pixels per cycle.
    const SPEED: f32 = 0.5;
    let mut mask = [BACKGROUND; mote::TOTAL_PIXELS];
    for p in particles.iter() {
        let x = ((n - p.creation_time) as f32 * SPEED) as usize;
        if x < mask.len() {
            mask[x] = p.color;
        }
    }
    mask
}

fn random_color() -> palette::rgb::LinSrgb {
    let between = rand::distributions::Uniform::new(0, 360);
    let mut rng = rand::thread_rng();
    let h = palette::RgbHue::<f32>::from_degrees(between.sample(&mut rng) as f32);
    let s = 1.0;
    let v = 0.5;
    palette::Hsv::new(h, s, v).into()
}

fn to_rgb(c: &palette::rgb::LinSrgb) -> rgb::RGB8 {
    rgb::RGB8 {
        r: (c.red * 255.0) as u8,
        g: (c.green * 255.0) as u8,
        b: (c.blue * 255.0) as u8,
    }
}
