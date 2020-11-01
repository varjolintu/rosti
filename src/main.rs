extern crate chrono;
extern crate clap;
extern crate curl;
extern crate serde_json;

use chrono::prelude::*;
use chrono::DateTime;
use clap::{Arg, App};
use curl::easy::{Easy2, Handler, WriteError};
use serde_json::Value;

static BASE_URL: &str = "https://www.posti.fi/henkiloasiakkaat/seuranta/api/shipments/";

struct Collector(Vec<u8>);

impl Handler for Collector {
    fn write(&mut self, data: &[u8]) -> Result<usize, WriteError> {
        self.0.extend_from_slice(data);
        Ok(data.len())
    }
}

fn get_event(event: &Value, code: &str) {
    if !event["locationName"].is_string() {
        return;
    }

    let desc = event["description"]["fi"].as_str().unwrap();
    let location = event["locationName"].as_str().unwrap();
    let timestamp = event["timestamp"].as_str().unwrap();
    let parsed_timestamp = DateTime::parse_from_rfc3339(timestamp).unwrap();
    
    // Add the offset
    let local: DateTime<Local> = Local::now();
    let offset = DateTime::offset(&local);
    let final_timestamp = parsed_timestamp.with_timezone(offset);

    println!("{}: {} {}, {}", code, desc, final_timestamp.format("%F %T"), location);
}

fn get_state(code: &str, multiple: &bool) {
    let url = format!("{}{}", BASE_URL, code);
    let mut easy = Easy2::new(Collector(Vec::new()));
    easy.get(true).unwrap();
    easy.url(&url).unwrap();
    easy.perform().unwrap();

    let response_code = easy.response_code().unwrap();
    if response_code != 200 {
        println!("Osoitetta ei löytynyt: {}", response_code);
        return;
    }

    let easy_ref = easy.get_ref();
    let raw_content = std::str::from_utf8(&easy_ref.0).unwrap();
    let content: Value = serde_json::from_str(raw_content).unwrap();
    
    if content["shipments"].as_array().unwrap().len() <= 0 {
        println!("Virheellinen seurantakoodi.");
        return;
    }

    for event in content["shipments"][0]["events"].as_array().unwrap() {
        get_event(event, code);
        if !multiple {
            break;
        }
    }
}

fn main() {
    let m = App::new("rosti")
            .version("0.1")
            .author("Sami Vänttinen")
            .about("Postin seurantakoodihaku")
        .arg(Arg::with_name("lista")
            .long("lista")
            .short("l")
            .takes_value(false)
            .help("Tulosta pitkä listaus"))
        .arg(Arg::with_name("KOODI")
            .required(true)
            .multiple(true)
            .help("Seurantakoodi"))
        .get_matches();

    let files: Vec<_> = m.values_of("KOODI").unwrap().collect();
    let files_length = files.len();
    let long_listing = m.is_present("lista");

    for f in files {
        get_state(&f, &long_listing);
        if long_listing && files_length > 1 {
            println!("");
        }
    }
}
