const CRLF: &str = "\r\n";
const ERROR_PREFIX: &str = "-";
const STRING_PREFIX: &str = "$";
const INT_PREFIX: &str = ":";
const JSON_PREFIX: &str = "?";
const ARRAY_PREFIX: &str = "*";
const POINT_PREFIX: &str = "!";

use serde_json::Value;

use std::error;

use crate::db::{ESValue};
use crate::{APP_VERSION, APP_AUTHORS, APP_HOMEPAGE};

pub trait JsonPrint {
    fn print_json(&self) -> Value;
}

pub trait GeoJsonFeature {
    fn geo_json_feature(&self) -> Value;
}


pub fn print_err(msg: &str) -> String {
    format!("{}{}{}", ERROR_PREFIX, msg, CRLF)
}

pub fn print_from_error(error: &dyn error::Error) -> String {
    format!("{}{}{}", ERROR_PREFIX, error.to_owned(), CRLF)
}
pub fn print_str(msg: &str) -> String {
    format!("+{}{}", msg, CRLF)
}

pub fn print_string_arr(arr: Vec<&String>) -> String {
    let mut str = String::new();
    str += &format!("{}{}{}", ARRAY_PREFIX, arr.len(), CRLF);
    for i in arr {
        str += &print_string(&i.to_string())
    };
    str
}

pub fn print_arr<T: ToString>(arr: Vec<T>) -> String {
    let mut str = String::new();
    str += &format!("{}{}{}", ARRAY_PREFIX, arr.len(), CRLF);
    for i in arr {
        str += &print_string(&i.to_string())
    };
    str
}

pub fn print_nested_arr<T: ToString>(arr: Vec<Vec<T>>) -> String {
    let mut str = String::new();
    str += &format!("{}{}{}", ARRAY_PREFIX, arr.len(), CRLF);
    for i in arr {
        str += &print_arr(i)
    };
    str
}

pub fn print_string(str: &String) -> String {
    format!("{}{}{}{}{}", STRING_PREFIX, str.len(), CRLF, str, CRLF)
}

pub fn print_integer(int: i64) -> String {
    format!("{}{}{}", INT_PREFIX, int, CRLF)
}

pub fn print_ok() -> String {
    print_str("OK")
}

pub fn print_pong() -> String {
    print_str("PONG")
}


pub fn build_geo_json<T: GeoJsonFeature>(f: &Vec<T>) -> Value {
    let features: Vec<Value> = f.iter().map(|m| {
        m.geo_json_feature()
    }).collect();

    json!(
        {
          "type": "FeatureCollection",
          "features": features
        }
    )
}

pub fn print_app_info() {
    let version = APP_VERSION;
    let authors = APP_AUTHORS;
    let homepage = APP_HOMEPAGE;
    println!(r##"
Ecanor version:{}  created by {}
{}
   ___      ___      ___      ___      ___      ___      ___
  /\  \    /\  \    /\  \    /\  \    /\__\    /\  \    /\  \
 /::\  \  /::\  \  /::\  \  /::\  \  /:| _|_  /::\  \  /::\  \
/::\:\__\/\:\:\__\/:/\:\__\/::\:\__\/::|/\__\/:/\:\__\/::\:\__\
\:\:\/  /\:\:\/__/\:\ \/__/\/\::/  /\/|::/  /\:\/:/  /\;:::/  /
 \:\/  /  \::/  /  \:\__\    /:/  /   |:/  /  \::/  /  |:\/__/
  \/__/    \/__/    \/__/    \/__/    \/__/    \/__/    \|__|
"##, version,authors,homepage);
}

#[test]
fn test_array_print() {
    let tester = "*2\r\n$3\r\nfoo\r\n$3\r\nbar\r\n";

    let foo = String::from("foo");
    let bar = String::from("bar");
    let sample_arr: Vec<&String> = vec![&foo, &bar];

    let sample = print_string_arr(sample_arr);

    assert_eq!(sample, tester)
}