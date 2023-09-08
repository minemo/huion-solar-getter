use std::fs;
use self::types::*;

pub mod types;

fn read_definitions() -> Root {
    let content = fs::read_to_string("./definitions.json").expect("Unable to read file");
    return serde_json::from_str(&content).expect("Invalid JSON supplied");
}

fn filter_category(data: &Vec<Const>, category: u8) -> Vec<&Const> {
    let filtered = data.iter().filter(|x| x.category == category).collect();
    return filtered;
}

pub fn gen_batdata(base_addr: u16, ident: u8) -> Vec<PVSignal> {
    let mut out = Vec::new();
    let defs = read_definitions();
    let scheme = defs.scheme.bat;

    for i in 0..scheme.len() {
        let b = scheme.get(i).expect("uh oh");
        let offset: u16 = base_addr + b.addr;
        let mut signal = PVSignal {
            data: PVSignalDataType::UNK(0),
            address: offset,
            length: b.len,
            name: format!("pack{}_{}", ident, b.name),
            unit: b.unit.to_string(),
            gain: b.gain,
            time: 0
        };
        match b.dtype.as_str() {
            "U16" => signal.data = PVSignalDataType::U16(0),
            "U32" => signal.data = PVSignalDataType::U32(0),
            "I16" => signal.data = PVSignalDataType::I16(0),
            "I32" => signal.data = PVSignalDataType::I32(0),
            "STR" => signal.data = PVSignalDataType::STR("".to_string()),
            _ => continue,
        }
        out.push(signal);
    }

    return out;
}

pub fn gen_pvdata(num_pvs: u8) -> Vec<PVString> {
    let mut pvs = Vec::new();
    for i in 0..num_pvs {
        let pv = PVString {
            voltage: PVSignal {
                data: PVSignalDataType::I16(0),
                address: 32016 + i as u16 * 2,
                length: 1,
                name: format!("pv_{}_voltage", i),
                unit: "V".to_string(),
                gain: 10,
                time: 0,
            },
            current: PVSignal {
                data: PVSignalDataType::I16(0),
                address: 32017 + i as u16 * 2,
                length: 1,
                name: format!("pv_{}_current", i),
                unit: "A".to_string(),
                gain: 100,
                time: 0,
            }
        };
        pvs.push(pv);
    }
    return pvs;
}

pub fn gen_constdata(category: u8) -> Vec<PVSignal> {
    let defs = read_definitions();
    let mut signals = Vec::new();
    for c in filter_category(&defs.const_field, category) {
        let mut signal = PVSignal {
            data: PVSignalDataType::UNK(0),
            address: c.addr,
            length: c.len,
            name: c.name.to_string(),
            unit: c.unit.to_string(),
            gain: c.gain,
            time: 0
        };
        match c.dtype.as_str() {
            "U16" => signal.data = PVSignalDataType::U16(0),
            "U32" => signal.data = PVSignalDataType::U32(0),
            "I16" => signal.data = PVSignalDataType::I16(0),
            "I32" => signal.data = PVSignalDataType::I32(0),
            "STR" => signal.data = PVSignalDataType::STR("".to_string()),
            _ => continue,
        }
        signals.push(signal);
    }
    return signals;
}

pub fn gen_storagedata() -> Vec<PVSignal> {
    let mut signals = gen_constdata(2);
    signals.append(&mut gen_batdata(38200, 0));
    signals.append(&mut gen_batdata(38242, 1));
    signals.append(&mut gen_batdata(38284, 2));
    return signals;
}