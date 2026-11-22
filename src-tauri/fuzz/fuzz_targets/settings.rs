#![no_main]

use std::sync::LazyLock;

use arbitrary::Arbitrary;
use clipsa_lib::settings::AppSettings;
use libfuzzer_sys::fuzz_target;
use serde_json::{Map, Value};

static KEYS: LazyLock<Vec<String>> =
    LazyLock::new(|| match serde_json::to_value(AppSettings::default()) {
        Ok(Value::Object(fields)) => fields.keys().cloned().collect(),
        _ => panic!("AppSettings didn't serialize to an object"),
    });

#[derive(Arbitrary, Debug)]
enum Key {
    Known(u8),
    Other(String),
}

#[derive(Arbitrary, Debug)]
enum Json {
    Null,
    Bool(bool),
    Int(i64),
    UInt(u64),
    Float(f64),
    Str(String),
    Array(Vec<Json>),
}

impl From<Json> for Value {
    fn from(json: Json) -> Self {
        match json {
            Json::Null => Value::Null,
            Json::Bool(value) => value.into(),
            Json::Int(value) => value.into(),
            Json::UInt(value) => value.into(),
            Json::Float(value) => value.into(),
            Json::Str(value) => value.into(),
            Json::Array(values) => values.into_iter().map(Value::from).collect(),
        }
    }
}

fuzz_target!(|fields: Vec<(Key, Json)>| {
    let patch: Map<String, Value> = fields
        .into_iter()
        .map(|(key, value)| {
            let key = match key {
                Key::Known(index) => KEYS[usize::from(index) % KEYS.len()].clone(),
                Key::Other(name) => name,
            };
            (key, value.into())
        })
        .collect();
    clipsa_lib::fuzz::settings_merge(Value::Object(patch));
});
