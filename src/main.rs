use std::time::{Duration,Instant};
use std::thread;
use std::env;

use tokio_modbus::prelude::*;

#[macro_use]
extern crate log;

mod datalogger;
mod parser;

struct ConnectionData {
    inverter_ip: String,
    redis_ip: String,
}

fn main() {
    env_logger::init();

    info!("Connecting to slave");

    let mut condata = ConnectionData{
        inverter_ip:"192.168.178.83:502".to_string(),
        redis_ip:"redis://192.168.178.109/".to_string()
    };

    match env::var("INV_IP") {
        Ok(v) => condata.inverter_ip = v,
        Err(_) => warn!("Environment variables not configured, using default"),
    };

    match env::var("RD_IP") {
        Ok(v) => condata.redis_ip = v,
        Err(_) => warn!("Environment variables not configured, using default"),
    }

    
    let mut ctx = sync::tcp::connect_slave_with_timeout(condata.inverter_ip.parse().unwrap(), Slave(1), Some(Duration::from_secs(5))).unwrap();
    let client = redis::Client::open(condata.redis_ip).unwrap();
    
    debug!("sleeping 1 second to make sure the slave is ready");

    thread::sleep(Duration::from_secs(1));

    info!("Trying to read device id from slave");

    // Read the device id to make sure everything is working as expected.
    let did: Vec<u16> = ctx.read_holding_registers(30000, 15).unwrap();

    // each register contains 2 chars
    let mut text = String::new();
    for i in 0..did.len() {
        text.push((did[i] >> 8) as u8 as char);
        text.push((did[i] & 0xFF) as u8 as char);
    }

    info!("Device ID: {}", text);

    let mut datalogger = datalogger::DataLogger::new(ctx, client);
    datalogger.init();

    loop {
        info!("Starting new gathering cycle");
        let readstart = Instant::now();
        datalogger.read_data();
        let readdur = readstart.elapsed();
        info!("Reading Registers took: {}s", readdur.as_secs());
        let writestart = Instant::now();
        datalogger.send_data("test_alt".to_string());
        let writedur = writestart.elapsed();
        info!("Writing to Database took: {}s", writedur.as_secs());

        thread::sleep(Duration::from_secs(90));
    }

}
