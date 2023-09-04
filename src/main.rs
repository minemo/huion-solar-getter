use std::borrow::Cow;
use std::time::Duration;
use std::thread;

use tokio_modbus::prelude::*;

#[macro_use]
extern crate log;

mod datalogger;

fn main() {
    env_logger::init();

    info!("Connecting to slave");

    let mut ctx = sync::tcp::connect_slave_with_timeout("192.168.178.83:502".parse().unwrap(), Slave(1), Some(Duration::from_secs(5))).unwrap();
    let client = redis::Client::open("redis://192.168.178.109/").unwrap();
    
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

    info!("Getting device list");

    let device_list: Response = ctx.call(Request::Custom(0x2b, Cow::Borrowed(&[0x0e, 03, 0x87]))).unwrap();

    info!("Device list: {:?}", device_list);

    let mut datalogger = datalogger::DataLogger::new(ctx, client);
    datalogger.init();

    loop {
        info!("Starting new gathering cycle");
        datalogger.read_data();
        datalogger.send_data("test".to_string());

        thread::sleep(Duration::from_secs(60));
    }

}
