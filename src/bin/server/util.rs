use serde_json::Value;
use crate::unit_conv::Units;

pub fn merge(a: &mut Value, b: &Value) {
    match (a, b) {
        (&mut Value::Object(ref mut a), &Value::Object(ref b)) => {
            for (k, v) in b {
                merge(a.entry(k.clone()).or_insert(Value::Null), v);
            }
        }
        (a, b) => {
            *a = b.clone();
        }
    }
}

pub const ALPHA_NUMERIC: [char; 62] = [
    '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', 'a', 'b', 'c', 'd', 'e', 'f', 'g',
    'h', 'i', 'j', 'k', 'l', 'm', 'n', 'o', 'p', 'q', 'r', 's', 't', 'u', 'v', 'w', 'x', 'y', 'z',
    'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'I', 'J', 'K', 'L', 'M', 'N', 'O', 'P', 'Q', 'R', 'S',
    'T', 'U', 'V', 'W', 'X', 'Y', 'Z',
];

pub fn is_numeric(num_str: &String) -> bool {
    if num_str.is_empty() {
        return false;
    }

    let first_char = num_str.chars().nth(0).unwrap();

    if !(first_char.is_numeric() || first_char == '-') {
        return false;
    }

    let is_num = num_str.parse::<f64>().is_ok();
    return is_num;
}

pub fn is_integer(num_str: &String) -> bool {
    if num_str.is_empty() {
        return false;
    }

    if num_str.len() > 20 {
        return false;
    }

    let first_char = num_str.chars().nth(0).unwrap();
    if !(first_char.is_numeric() || first_char == '-') {
        return false;
    }

    let is_num = num_str.parse::<i64>().is_ok();
    return is_num;
}

pub fn is_json(json_str: &String) -> bool {
    let is_json = serde_json::from_slice::<Value>(json_str.as_bytes()).is_ok();
    return is_json;
}

pub fn get_distance(a: (f64, f64), b: (f64, f64)) -> f64 {
    let loc_a = Location { latitude: a.0, longitude: a.1 };
    let loc_b = Location { latitude: b.0, longitude: b.1 };

    haversine_distance(loc_a, loc_b, Units::Meters)
}

pub struct Location {
    pub latitude: f64,
    pub longitude: f64,
}


pub fn haversine_distance(start: Location, end: Location, units: Units) -> f64 {
    let kilometers: f64 = 6371.0;
    let miles: f64 = 3960.0;
    let meters: f64 = 6_371_000.000000;
    let mut r: f64 = 0.0;

    match units {
        Units::Miles => r = miles,
        Units::Kilometers => r = kilometers,
        Units::Meters => r = meters
    }

    let d_lat: f64 = (end.latitude - start.latitude).to_radians();
    let d_lon: f64 = (end.longitude - start.longitude).to_radians();
    let lat1: f64 = (start.latitude).to_radians();
    let lat2: f64 = (end.latitude).to_radians();

    let a: f64 = ((d_lat / 2.0).sin()) * ((d_lat / 2.0).sin()) + ((d_lon / 2.0).sin()) * ((d_lon / 2.0).sin()) * (lat1.cos()) * (lat2.cos());
    let c: f64 = 2.0 * ((a.sqrt()).atan2((1.0 - a).sqrt()));

    return r * c;
}


#[cfg(test)]
mod tests {
    extern crate bytes;

    use redis_protocol::prelude::*;

    use super::*;

    use regex::Regex;


    pub fn is_numeric_with_regex(num_str: &String) -> bool {
        if num_str.is_empty() {
            return false;
        }
        let re = Regex::new(r"^(([1-9]*)|(([1-9]*)\.([0-9]*)))$").unwrap();
        re.is_match(num_str)
    }

    #[test]
    fn test_merge() {
        let mut a = json!({
        "person" : {
            "firstName" : "John",
            "lastName" : "Doe"
        },
        "cities":[ "london", "paris" ]
    });

        let b = json!({
        "title": "This is another title",
        "payment": 20,
        "person" : {
            "firstName" : "Jane"
        },
        "cities": null
    });

        merge(&mut a, &b);
        println!("{:#}", a);
    }

    #[test]
    fn test_redis_parser() {
        let buf = "*3\r\n$3\r\nFoo\r\n$-1\r\n$3\r\nBar\r\n".into();

        let (frame, consumed) = match decode_bytes(&buf) {
            Ok((f, c)) => (f, c),
            Err(e) => panic!("Error parsing bytes: {:?}", e)
        };

        if let Some(frame) = frame {
            println!("{:?}", frame)
        } else {
            println!("Incomplete frame, parsed {} bytes", consumed);
        }
    }


    #[test]
    fn test_numeric() {
        let float_str = String::from("8.5");
        assert!(is_numeric(&float_str));
        let float_str = String::from("-8.5");
        assert!(is_numeric(&float_str));
        let float_str = String::from("-8.5");
        assert!(true, is_numeric_with_regex(&float_str));
    }
}

