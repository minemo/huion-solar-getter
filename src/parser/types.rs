use serde_derive::Deserialize;
use serde_derive::Serialize;

#[derive(Debug, Clone)]
pub enum PVSignalDataType {
    U16(u16),
    I16(i16),
    U32(u32),
    I32(i32),
    STR(String),
    UNK(u16),
}
#[derive(Debug, Clone)]
pub struct PVSignal {
    pub data: PVSignalDataType,
    pub address: u16,
    pub length: u16,
    pub name: String,
    pub unit: String,
    pub gain: u16,
    pub time: i64,
}

#[derive(Debug)]
pub struct PVString {
    pub voltage: PVSignal,
    pub current: PVSignal,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Root {
    #[serde(rename = "const")]
    pub const_field: Vec<Const>,
    pub scheme: Scheme,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Const {
    pub dtype: String,
    pub addr: u16,
    pub len: u16,
    pub gain: u16,
    pub name: String,
    pub unit: String,
    pub category: u8,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Scheme {
    pub bat: Vec<Bat>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Bat {
    pub dtype: String,
    pub addr: u16,
    pub len: u16,
    pub gain: u16,
    pub name: String,
    pub unit: String,
}