const CRLF: &str = "\r\n";
const ERROR_PREFIX: &str = "-";
const STRING_PREFIX: &str = "$";
const INT_PREFIX: &str = ":";
const JSON_PREFIX: &str = "?";
const ARRAY_PREFIX: &str = "*";
const POINT_PREFIX: &str = "!";

use std::error;

use crate::db::{ESRecord, DataType};
use serde::de::Unexpected::Str;


pub fn print_err(msg: &str) -> String {
    format!("{}{}{}", ERROR_PREFIX, msg, CRLF)
}

pub fn print_from_error(error: &dyn error::Error) -> String {
    format!("{}{}{}", ERROR_PREFIX, error.to_owned(), CRLF)
}

pub fn print_record(record: &ESRecord) -> String {
    let prefix = match record.data_type {
        DataType::String => STRING_PREFIX,
        DataType::Integer => INT_PREFIX,
    };

    if prefix == STRING_PREFIX {
       return print_string(&record.value)
    }

    format!("{}{}{}", prefix, record.value.to_owned(), CRLF)
}

pub fn print_str(msg: &str) -> String {
    format!("+{}{}",msg, CRLF)
}

pub fn print_string_arr(arr : Vec<&String>) -> String {
    let mut str = String::new();
    str += &format!("{}{}{}",ARRAY_PREFIX,arr.len(),CRLF);
    for i in arr {
        str += &print_string(i)
    };
    str
}

/*
pub fn print_str_arr(arr : Vec<&str>){

}*/

pub fn print_string(str: &String) -> String {
    format!("{}{}{}{}{}", STRING_PREFIX, str.len(), CRLF, str, CRLF)
}

pub fn print_ok() -> String {
    print_str("OK")
}

/* @todo implement later
pub fn print_record_as_u8byte_arr(record : &ESRecord) -> String {
    format!("{}{}{}",msg,DELIMITER)
}
*/

#[test]
fn test_array_print() {
    let tester = "*2\r\n$3\r\nfoo\r\n$3\r\nbar\r\n";

    let foo = String::from("foo");
    let bar = String::from("bar");
    let sample_arr : Vec<&String> = vec![&foo,&bar];

    let sample = print_string_arr(sample_arr);

    assert_eq!(sample,tester)
}