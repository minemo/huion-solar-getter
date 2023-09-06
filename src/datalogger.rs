use redis::ToRedisArgs;
use tokio_modbus::{client::sync, prelude::SyncReader};
use rand::prelude::*;

extern crate redis;

#[derive(Debug, Clone)]
enum PVSignalDataType {
    U16(u16),
    I16(i16),
    U32(u32),
    I32(i32),
    STR(String)
}
#[derive(Debug, Clone)]
pub struct PVSignal {
    data: PVSignalDataType,
    address: u16,
    length: u16,
    name: String,
    unit: String,
    gain: u16,
}

#[derive(Debug)]
pub struct PVString {
    voltage: PVSignal,
    current: PVSignal,
}

#[derive(Debug)]
pub struct DataLogger {
    ctx: sync::Context,
    redis_client: redis::Client,
    pvs: Vec<PVString>,
    general_data: Vec<PVSignal>,
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

impl DataLogger {
    pub fn new(ctx: sync::Context, redis: redis::Client) -> DataLogger {
        DataLogger {
            ctx,
            redis_client: redis,
            pvs: Vec::new(),
            general_data: Vec::new(),
            storage_data: Vec::new(),
        }
    }

    pub fn init(&mut self) {
        self._init_pvs();
        self._init_general_data();
        self._init_storage_data();
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
        let hastimeseries: Vec<String> = redis::cmd("TS.QUERYINDEX").arg("type=solar").query(&mut con).unwrap();
        if hastimeseries.len() != (self.general_data.len() + self.storage_data.len() + self.pvs.len() * 2) {
            // create timeseries for keys
            debug!("Creating timeseries for keys");
            for i in 0..self.general_data.len() {
                let _: () = redis::cmd("TS.CREATE").arg(format!("{}:general:{}", base_key, self.general_data[i].name)).arg("LABELS").arg("type").arg("solar").arg("data").arg("general").query(&mut con).unwrap();
            }

            for i in 0..self.storage_data.len() {
                let _: () = redis::cmd("TS.CREATE").arg(format!("{}:storage:{}", base_key, self.storage_data[i].name)).arg("LABELS").arg("type").arg("solar").arg("data").arg("storage").query(&mut con).unwrap();
            }

            for i in 0..self.pvs.len() {
                let _: () = redis::cmd("TS.CREATE").arg(format!("{}:pv:{}", base_key, self.pvs[i].voltage.name)).arg("LABELS").arg("type").arg("solar").arg("data").arg("pv_volt").query(&mut con).unwrap();
                let _: () = redis::cmd("TS.CREATE").arg(format!("{}:pv:{}", base_key, self.pvs[i].current.name)).arg("LABELS").arg("type").arg("solar").arg("data").arg("pv_curr").query(&mut con).unwrap();
            }
        }

        // adding a lookup-table to database if it doesnt already exist
        debug!("Updating/adding lookup table");
        let mut creator = redis::cmd("HSET");
        let alldata: Vec<PVSignal> = [self.general_data.as_slice(), self.storage_data.as_slice(), self.pvs.iter().map(|x| [x.current.clone(), x.voltage.clone()]).flatten().collect::<Vec<PVSignal>>().as_slice()].concat();
        for d in alldata.iter() {
            creator = creator.arg(&[&d.name, &d.unit]).to_owned();
        }
        let _: () = creator.query(&mut con).unwrap();

        info!("Saving data to redis");

        // save data to redis
        let ts = chrono::Utc::now().timestamp_millis();
        for i in 0..self.general_data.len() {
            // filter out strings
            match self.general_data[i].data {
                PVSignalDataType::U16(v) => {
                    let _: () = redis::cmd("TS.ADD").arg(format!("{}:general:{}", base_key, self.general_data[i].name)).arg(ts).arg((v.clone() as f32)/(self.general_data[i].gain as f32)).query(&mut con).unwrap();
                },
                PVSignalDataType::I16(v) => {
                    let _: () = redis::cmd("TS.ADD").arg(format!("{}:general:{}", base_key, self.general_data[i].name)).arg(ts).arg((v.clone() as f32)/(self.general_data[i].gain as f32)).query(&mut con).unwrap();
                },
                PVSignalDataType::U32(v) => {
                    let _: () = redis::cmd("TS.ADD").arg(format!("{}:general:{}", base_key, self.general_data[i].name)).arg(ts).arg((v.clone() as f32)/(self.general_data[i].gain as f32)).query(&mut con).unwrap();
                },
                PVSignalDataType::I32(v) => {
                    let _: () = redis::cmd("TS.ADD").arg(format!("{}:general:{}", base_key, self.general_data[i].name)).arg(ts).arg((v.clone() as f32)/(self.general_data[i].gain as f32)).query(&mut con).unwrap();
                },
                _ => {
                    debug!("Skipping string: {}", self.general_data[i].name);
                },
            }
        }

        for i in 0..self.storage_data.len() {
            match self.storage_data[i].data {
                PVSignalDataType::U16(v) => {
                    let _: () = redis::cmd("TS.ADD").arg(format!("{}:storage:{}", base_key, self.storage_data[i].name)).arg(ts).arg((v.clone() as f32)/(self.storage_data[i].gain as f32)).query(&mut con).unwrap();
                },
                PVSignalDataType::I16(v) => {
                    let _: () = redis::cmd("TS.ADD").arg(format!("{}:storage:{}", base_key, self.storage_data[i].name)).arg(ts).arg((v.clone() as f32)/(self.storage_data[i].gain as f32)).query(&mut con).unwrap();
                },
                PVSignalDataType::U32(v) => {
                    let _: () = redis::cmd("TS.ADD").arg(format!("{}:storage:{}", base_key, self.storage_data[i].name)).arg(ts).arg((v.clone() as f32)/(self.storage_data[i].gain as f32)).query(&mut con).unwrap();
                },
                PVSignalDataType::I32(v) => {
                    let _: () = redis::cmd("TS.ADD").arg(format!("{}:storage:{}", base_key, self.storage_data[i].name)).arg(ts).arg((v.clone() as f32)/(self.storage_data[i].gain as f32)).query(&mut con).unwrap();
                },
                _ => {
                    debug!("Skipping string: {}", self.general_data[i].name);
                },
            }
        }

        for i in 0..self.pvs.len() {
            match self.pvs[i].voltage.data {
                PVSignalDataType::I16(v) => {
                    let _: () = redis::cmd("TS.ADD").arg(format!("{}:pv:{}", base_key, self.pvs[i].voltage.name)).arg(ts).arg((v.clone() as f32)/(self.pvs[i].voltage.gain as f32)).query(&mut con).unwrap();
                },
                _ => {
                    debug!("Skipping string: {}", self.pvs[i].voltage.name);
                },
            }
            match self.pvs[i].current.data {
                PVSignalDataType::I16(v) => {
                    let _: () = redis::cmd("TS.ADD").arg(format!("{}:pv:{}", base_key, self.pvs[i].current.name)).arg(ts).arg((v.clone() as f32)/(self.pvs[i].current.gain as f32)).query(&mut con).unwrap();
                },
                _ => {
                    debug!("Skipping string: {}", self.pvs[i].current.name);
                },
            }
        
        }

        // close the connection
        let _: () = redis::cmd("QUIT").query(&mut con).unwrap();
    }

    pub fn get_pvs(&self) -> &Vec<PVString> {
        &self.pvs
    }

    pub fn get_general_data(&self) -> &Vec<PVSignal> {
        &self.general_data
    }

    pub fn get_storage_data(&self) -> &Vec<PVSignal> {
        &self.storage_data
    }

    fn _init_pvs(&mut self) {
        let num_pvs = self._get_num_pvs();
        for i in 0..num_pvs {
            let pv = PVString {
                voltage: PVSignal {
                    data: PVSignalDataType::I16(0),
                    address: 32016 + i * 2,
                    length: 1,
                    name: format!("pv_{}_voltage", i),
                    unit: "V".to_string(),
                    gain: 10,
                },
                current: PVSignal {
                    data: PVSignalDataType::I16(0),
                    address: 32017 + i * 2,
                    length: 1,
                    name: format!("pv_{}_current", i),
                    unit: "A".to_string(),
                    gain: 100,
                }
            };
            self.pvs.push(pv);
        }
    }

    fn _init_general_data(&mut self) {
        //? Add entries manually here, because they are too different to be generated
        self.general_data.push(PVSignal {
            data: PVSignalDataType::STR("".to_string()),
            address: 30000,
            length: 15,
            name: "model_ident".to_string(),
            unit: "".to_string(),
            gain: 1,
        });
        self.general_data.push(PVSignal {
            data: PVSignalDataType::U32(0),
            address: 30073,
            length: 2,
            name: "rated_power".to_string(),
            unit: "kW".to_string(),
            gain: 1000,
        });
        self.general_data.push(PVSignal {
            data: PVSignalDataType::U32(0),
            address: 30075,
            length: 2,
            name: "maximum_active_power".to_string(),
            unit: "kW".to_string(),
            gain: 1000,
        });
        self.general_data.push(PVSignal {
            data: PVSignalDataType::U32(0),
            address: 30077,
            length: 2,
            name: "maximum_apparent_power".to_string(),
            unit: "kVA".to_string(),
            gain: 1000,
        });
        self.general_data.push(PVSignal {
            data: PVSignalDataType::I32(0),
            address: 32064,
            length: 2,
            name: "input_power".to_string(),
            unit: "kW".to_string(),
            gain: 1000,
        });
        self.general_data.push(PVSignal {
            data: PVSignalDataType::U16(0),
            address: 32066,
            length: 1,
            name: "grid_voltage".to_string(),
            unit: "V".to_string(),
            gain: 10,
        });
        self.general_data.push(PVSignal {
            data: PVSignalDataType::I32(0),
            address: 32072,
            length: 2,
            name: "grid_current".to_string(),
            unit: "V".to_string(),
            gain: 1000,
        });
        self.general_data.push(PVSignal {
            data: PVSignalDataType::I32(0),
            address: 32078,
            length: 2,
            name: "peak_active_day".to_string(),
            unit: "kW".to_string(),
            gain: 1000,
        });
        self.general_data.push(PVSignal {
            data: PVSignalDataType::I32(0),
            address: 32080,
            length: 2,
            name: "active_power".to_string(),
            unit: "kW".to_string(),
            gain: 1000,
        });
        self.general_data.push(PVSignal {
            data: PVSignalDataType::I32(0),
            address: 32082,
            length: 2,
            name: "reactive_power".to_string(),
            unit: "kVar".to_string(),
            gain: 1000,
        });
        self.general_data.push(PVSignal {
            data: PVSignalDataType::U16(0),
            address: 32085,
            length: 1,
            name: "grid_frequency".to_string(),
            unit: "Hz".to_string(),
            gain: 100,
        });
        self.general_data.push(PVSignal {
            data: PVSignalDataType::U16(0),
            address: 32086,
            length: 1,
            name: "efficiency".to_string(),
            unit: "kW".to_string(),
            gain: 100,
        });
        self.general_data.push(PVSignal {
            data: PVSignalDataType::I16(0),
            address: 32087,
            length: 1,
            name: "temp".to_string(),
            unit: "C".to_string(),
            gain: 10,
        });
        self.general_data.push(PVSignal {
            data: PVSignalDataType::U32(0),
            address: 32106,
            length: 2,
            name: "acc_energy_yield".to_string(),
            unit: "kWh".to_string(),
            gain: 100,
        });
        self.general_data.push(PVSignal {
            data: PVSignalDataType::U32(0),
            address: 32114,
            length: 2,
            name: "daily_energy_yield".to_string(),
            unit: "kWh".to_string(),
            gain: 100,
        });
        self.general_data.push(PVSignal {
            data: PVSignalDataType::U32(0),
            address: 40000,
            length: 2,
            name: "time".to_string(),
            unit: "".to_string(),
            gain: 1,
        });
    }

    fn _init_storage_data(&mut self) {
        self.storage_data.push(PVSignal {
            data: PVSignalDataType::I32(0),
            address: 37001,
            length: 2,
            name: "charge_discharge_power".to_string(),
            unit: "W".to_string(),
            gain: 1,
        });
        self.storage_data.push(PVSignal {
            data: PVSignalDataType::U32(0),
            address: 37015,
            length: 2,
            name: "day_charge_capacity".to_string(),
            unit: "kWh".to_string(),
            gain: 100,
        });
        self.storage_data.push(PVSignal {
            data: PVSignalDataType::U32(0),
            address: 37017,
            length: 2,
            name: "day_discharge_capacity".to_string(),
            unit: "kWh".to_string(),
            gain: 100,
        });
        self.storage_data.push(PVSignal {
            data: PVSignalDataType::I32(0),
            address: 37113,
            length: 2,
            name: "active_power".to_string(),
            unit: "W".to_string(),
            gain: 1,
        });
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
            data.append(&mut tmp);
        }
        for i in 0..self.general_data.len() {
            match self.general_data[i].data {
                PVSignalDataType::U16(_) => {
                    self.general_data[i].data = PVSignalDataType::U16(data[i]);
                },
                PVSignalDataType::I16(_) => {
                    self.general_data[i].data = PVSignalDataType::I16(data[i] as i16);
                },
                PVSignalDataType::U32(_) => {
                    self.general_data[i].data = PVSignalDataType::U32((data[i] as u32) << 16 | data[i+1] as u32);
                },
                PVSignalDataType::I32(_) => {
                    self.general_data[i].data = PVSignalDataType::I32(((data[i] as i32) << 16 | data[i+1] as i32) as i32);
                },
                PVSignalDataType::STR(_) => {
                    let mut text = String::new();
                    for j in 0..self.general_data[i].length {
                        text.push((data[i+j as usize] >> 8) as u8 as char);
                        text.push((data[i+j as usize] & 0xFF) as u8 as char);
                    }
                    self.general_data[i].data = PVSignalDataType::STR(text);
                },
            }
        }
    }

    fn _read_storage_data(&mut self) {
        let mut data: Vec<u16> = Vec::new();
        for i in 0..self.storage_data.len() {
            debug!("Reading data for: {}", self.storage_data[i].name);
            let mut tmp: Vec<u16> = self.ctx.read_holding_registers(self.storage_data[i].address, self.storage_data[i].length).unwrap();
            data.append(&mut tmp);
        }
        for i in 0..self.storage_data.len() {
            match self.storage_data[i].data {
                PVSignalDataType::U16(_) => {
                    self.storage_data[i].data = PVSignalDataType::U16(data[i]);
                },
                PVSignalDataType::I16(_) => {
                    self.storage_data[i].data = PVSignalDataType::I16(data[i] as i16);
                },
                PVSignalDataType::U32(_) => {
                    self.storage_data[i].data = PVSignalDataType::U32((data[i] as u32) << 16 | data[i+1] as u32);
                },
                PVSignalDataType::I32(_) => {
                    self.storage_data[i].data = PVSignalDataType::I32(((data[i] as i32) << 16 | data[i+1] as i32) as i32);
                },
                PVSignalDataType::STR(_) => {
                    let mut text = String::new();
                    for j in 0..self.storage_data[i].length {
                        text.push((data[i+j as usize] >> 8) as u8 as char);
                        text.push((data[i+j as usize] & 0xFF) as u8 as char);
                    }
                    self.storage_data[i].data = PVSignalDataType::STR(text);
                },
            }
        }
    }

    fn _read_pv_data(&mut self) {
        let mut data: Vec<u16> = Vec::new();
        for i in 0..self.pvs.len() {
            debug!("Reading data for: {}", self.pvs[i].voltage.name);
            let mut tmp: Vec<u16> = self.ctx.read_holding_registers(self.pvs[i].voltage.address, self.pvs[i].voltage.length).unwrap();
            data.append(&mut tmp);
            debug!("Reading data for: {}", self.pvs[i].current.name);
            let mut tmp: Vec<u16> = self.ctx.read_holding_registers(self.pvs[i].current.address, self.pvs[i].current.length).unwrap();
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
        pvs[0]
    }
}