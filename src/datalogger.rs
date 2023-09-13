use redis::ToRedisArgs;
use tokio_modbus::{client::sync, prelude::SyncReader};
use rand::prelude::*;
use crate::parser::{types::*, gen_constdata, gen_pvdata, gen_storagedata};

extern crate redis;

#[derive(Debug)]
pub struct DataLogger {
    ctx: sync::Context,
    redis_client: redis::Client,
    pvs: Vec<PVString>,
    general_data: Vec<PVSignal>,
    pgs_data: Vec<PVSignal>,
    storage_data: Vec<PVSignal>,
}

impl ToRedisArgs for PVSignalDataType {
    fn write_redis_args<W>(&self, out: &mut W)
    where
        W: ?Sized + redis::RedisWrite,
        Self: Sized, {
        match self {
            PVSignalDataType::U16(x) => x.write_redis_args(out),
            PVSignalDataType::I16(x) => x.write_redis_args(out),
            PVSignalDataType::U32(x) => x.write_redis_args(out),
            PVSignalDataType::I32(x) => x.write_redis_args(out),
            PVSignalDataType::STR(x) => x.write_redis_args(out),
            PVSignalDataType::UNK(x) => x.write_redis_args(out),
        }
    }

    fn to_redis_args(&self) -> Vec<Vec<u8>> {
        let mut out = Vec::new();
        self.write_redis_args(&mut out);
        out
    }

    fn describe_numeric_behavior(&self) -> redis::NumericBehavior {
        redis::NumericBehavior::NonNumeric
    }

    fn is_single_arg(&self) -> bool {
        true
    }
}

fn sort_data_redis(i: usize, base_key: &String, cat_key: String, base_data: &Vec<PVSignal>, con: &mut redis::Connection) {
    match base_data[i].data {
        PVSignalDataType::U16(v) => {
            let _: () = redis::cmd("TS.ADD").arg(format!("{}:{}:{}", base_key, cat_key, base_data[i].name)).arg(base_data[i].time).arg(v.clone()).query(con).unwrap();
        },
        PVSignalDataType::I16(v) => {
            let _: () = redis::cmd("TS.ADD").arg(format!("{}:{}:{}", base_key, cat_key, base_data[i].name)).arg(base_data[i].time).arg(v.clone()).query(con).unwrap();
        },
        PVSignalDataType::U32(v) => {
            let _: () = redis::cmd("TS.ADD").arg(format!("{}:{}:{}", base_key, cat_key, base_data[i].name)).arg(base_data[i].time).arg(v.clone()).query(con).unwrap();
        },
        PVSignalDataType::I32(v) => {
            let _: () = redis::cmd("TS.ADD").arg(format!("{}:{}:{}", base_key, cat_key, base_data[i].name)).arg(base_data[i].time).arg(v.clone()).query(con).unwrap();
        },
        PVSignalDataType::STR(_) => {
            debug!("Skipping string: {}", base_data[i].name);
        },
        PVSignalDataType::UNK(_) => {
            warn!("Got an unknown: {}", base_data[i].name);
        }
    }
}

fn sort_data_store(base_data: &mut Vec<PVSignal>, data: Vec<u16>) {
    for i in 0..base_data.len() {
        match base_data[i].data {
            PVSignalDataType::U16(_) => {
                base_data[i].data = PVSignalDataType::U16(data[i]);
            },
            PVSignalDataType::I16(_) => {
                base_data[i].data = PVSignalDataType::I16(data[i] as i16);
            },
            PVSignalDataType::U32(_) => {
                base_data[i].data = PVSignalDataType::U32((data[i] as u32) << 16 | data[i+1] as u32);
            },
            PVSignalDataType::I32(_) => {
                base_data[i].data = PVSignalDataType::I32(((data[i] as u32) << 16 | data[i+1] as u32) as i32);
            },
            PVSignalDataType::STR(_) => {
                let mut text = String::new();
                for j in 0..base_data[i].length {
                    text.push((data[i+j as usize] >> 8) as u8 as char);
                    text.push((data[i+j as usize] & 0xFF) as u8 as char);
                }
                base_data[i].data = PVSignalDataType::STR(text);
            },
            PVSignalDataType::UNK(_) => base_data[i].data = PVSignalDataType::UNK(data[i]),
        }
    }
}

impl DataLogger {
    pub fn new(ctx: sync::Context, redis: redis::Client) -> DataLogger {
        DataLogger {
            ctx,
            redis_client: redis,
            pvs: Vec::new(),
            general_data: Vec::new(),
            pgs_data: Vec::new(),
            storage_data: Vec::new(),
        }
    }

    pub fn init(&mut self) {
        let num_pvs = self._get_num_pvs() as u8;
        self.pvs = gen_pvdata(num_pvs);
        self.general_data = gen_constdata(0);
        // self.pgs_data = gen_constdata(1);
        self.storage_data = gen_storagedata();
        self._test_redis();
    }

    pub fn read_data(&mut self) {
        info!("Reading data from inverter");
        self._read_general_data();
        self._read_storage_data();
        self._read_pv_data();
    }

    pub fn send_data(&mut self, base_key: String) {
        let mut con = self.redis_client.get_connection().unwrap();
        // save data to redis using timeseries
        // check if timeseries exists (returns Array with time series for each key or empty array)
        let hastimeseries: Vec<String> = redis::cmd("TS.QUERYINDEX").arg(format!("base={}",base_key)).query(&mut con).unwrap();
        debug!("{} + {} + {} + {} Values",self.general_data.len(), self.pgs_data.len(), self.storage_data.len(), self.pvs.len() * 2);
        if hastimeseries.len() != (self.general_data.len() + self.storage_data.len() + self.pvs.len() * 2) {
            // create timeseries for keys
            debug!("Creating timeseries for keys");
            for i in 0..self.general_data.len() {
                let _: () = redis::cmd("TS.CREATE").arg(format!("{}:general:{}", base_key.to_owned(), self.general_data[i].name)).arg("LABELS").arg("base").arg(base_key.to_owned()).arg("type").arg("solar").arg("data").arg("general").query(&mut con).unwrap();
            }

            for i in 0..self.storage_data.len() {
                let _: () = redis::cmd("TS.CREATE").arg(format!("{}:storage:{}", base_key.to_owned(), self.storage_data[i].name)).arg("LABELS").arg("base").arg(base_key.to_owned()).arg("type").arg("solar").arg("data").arg("storage").query(&mut con).unwrap();
            }

            for i in 0..self.pgs_data.len() {
                let _: () = redis::cmd("TS.CREATE").arg(format!("{}:pgs:{}", base_key.to_owned(), self.pgs_data[i].name)).arg("LABELS").arg("base").arg(base_key.to_owned()).arg("type").arg("solar").arg("data").arg("storage").query(&mut con).unwrap();
            }

            for i in 0..self.pvs.len() {
                let _: () = redis::cmd("TS.CREATE").arg(format!("{}:pv:{}", base_key.to_owned(), self.pvs[i].voltage.name)).arg("LABELS").arg("base").arg(base_key.to_owned()).arg("type").arg("solar").arg("data").arg("pv_volt").query(&mut con).unwrap();
                let _: () = redis::cmd("TS.CREATE").arg(format!("{}:pv:{}", base_key.to_owned(), self.pvs[i].current.name)).arg("LABELS").arg("base").arg(base_key.to_owned()).arg("type").arg("solar").arg("data").arg("pv_curr").query(&mut con).unwrap();
            }
        }

        // adding a lookup-table to database if it doesnt already exist
        debug!("Updating/adding lookup table");
        let mut creator = redis::cmd("HSET");
        creator.arg(format!("{}:lookup",base_key));
        let alldata: Vec<PVSignal> = [self.general_data.as_slice(), self.storage_data.as_slice(), self.pvs.iter().map(|x| [x.current.clone(), x.voltage.clone()]).flatten().collect::<Vec<PVSignal>>().as_slice()].concat();
        for d in alldata.iter() {
            creator.arg(&d.name).arg(&d.unit).arg(&d.gain);
        }
        let _: () = creator.query(&mut con).unwrap();

        info!("Saving data to redis");

        // save data to redis
        for i in 0..self.general_data.len() {
            sort_data_redis(i, &base_key, "general".to_string(), &self.general_data, &mut con);
        }

        for i in 0..self.storage_data.len() {
            sort_data_redis(i, &base_key, "storage".to_string(), &self.storage_data, &mut con);
        }

        for i in 0..self.pgs_data.len() {
            sort_data_redis(i, &base_key, "pgs".to_string(), &self.pgs_data, &mut con);
        }

        for i in 0..self.pvs.len() {
            match self.pvs[i].voltage.data {
                PVSignalDataType::I16(v) => {
                    let _: () = redis::cmd("TS.ADD").arg(format!("{}:pv:{}", base_key, self.pvs[i].voltage.name)).arg(self.pvs[i].voltage.time).arg(v.clone()).query(&mut con).unwrap();
                },
                _ => {
                    debug!("Skipping string: {}", self.pvs[i].voltage.name);
                },
            }
            match self.pvs[i].current.data {
                PVSignalDataType::I16(v) => {
                    let _: () = redis::cmd("TS.ADD").arg(format!("{}:pv:{}", base_key, self.pvs[i].current.name)).arg(self.pvs[i].current.time).arg(v.clone()).query(&mut con).unwrap();
                },
                _ => {
                    debug!("Skipping string: {}", self.pvs[i].current.name);
                },
            }
        
        }

        // close the connection
        let _: () = redis::cmd("QUIT").query(&mut con).unwrap();
    }


    pub fn _get_pvs(&self) -> &Vec<PVString> {
        &self.pvs
    }

    pub fn _get_general_data(&self) -> &Vec<PVSignal> {
        &self.general_data
    }

    pub fn _get_storage_data(&self) -> &Vec<PVSignal> {
        &self.storage_data
    }

    fn _test_redis(&mut self) {
        let mut con = self.redis_client.get_connection().unwrap();

        // test if redis is reachable and usable
        let secret = rand::thread_rng().gen::<u32>();
        let _: () = redis::cmd("SET").arg("ping").arg(secret).query(&mut con).unwrap();
        let result: u32 = redis::cmd("GET").arg("ping").query(&mut con).unwrap();
        assert_eq!(result, secret);
        debug!("Redis test successful");
        let _: () = redis::cmd("DEL").arg("ping").query(&mut con).unwrap();

        // close the connection
        let _: () = redis::cmd("QUIT").query(&mut con).unwrap();
    }

    fn _read_general_data(&mut self) {
        let mut data: Vec<u16> = Vec::new();
        for i in 0..self.general_data.len() {
            debug!("Reading data for: {}", self.general_data[i].name);
            let mut tmp: Vec<u16> = self.ctx.read_holding_registers(self.general_data[i].address, self.general_data[i].length).unwrap();
            self.general_data[i].time = chrono::Utc::now().timestamp_millis();
            data.append(&mut tmp);
        }
        sort_data_store(&mut self.general_data,data);
    }

    fn _read_storage_data(&mut self) {
        let mut data: Vec<u16> = Vec::new();
        for i in 0..self.storage_data.len() {
            debug!("Reading data for: {}", self.storage_data[i].name);
            let mut tmp: Vec<u16> = self.ctx.read_holding_registers(self.storage_data[i].address, self.storage_data[i].length).unwrap();
            self.storage_data[i].time = chrono::Utc::now().timestamp_millis();
            data.append(&mut tmp);
        }
        sort_data_store(&mut self.storage_data,data);
    }

    fn _read_pv_data(&mut self) {
        let mut data: Vec<u16> = Vec::new();
        for i in 0..self.pvs.len() {
            debug!("Reading data for: {}", self.pvs[i].voltage.name);
            let mut tmp: Vec<u16> = self.ctx.read_holding_registers(self.pvs[i].voltage.address, self.pvs[i].voltage.length).unwrap();
            self.pvs[i].voltage.time = chrono::Utc::now().timestamp_millis();
            data.append(&mut tmp);
            debug!("Reading data for: {}", self.pvs[i].current.name);
            let mut tmp: Vec<u16> = self.ctx.read_holding_registers(self.pvs[i].current.address, self.pvs[i].current.length).unwrap();
            self.pvs[i].current.time = chrono::Utc::now().timestamp_millis();
            data.append(&mut tmp);
        }
        for i in 0..self.pvs.len() {
            match self.pvs[i].voltage.data {
                PVSignalDataType::I16(_) => {
                    self.pvs[i].voltage.data = PVSignalDataType::I16(data[i*2] as i16);
                },
                _ => {
                    error!("Wrong data type for voltage: {:?}", self.pvs[i].voltage.data)
                },
            }
            match self.pvs[i].current.data {
                PVSignalDataType::I16(_) => {
                    self.pvs[i].current.data = PVSignalDataType::I16(data[i*2+1] as i16);
                },
                _ => {
                    error!("Wrong data type for current: {:?}", self.pvs[i].current.data)
                },
            }
        }
    }

    fn _get_num_pvs(&mut self) -> u16 {
        let pvs: Vec<u16> = self.ctx.read_holding_registers(30071, 1).unwrap();
        return pvs[0];
    }
}