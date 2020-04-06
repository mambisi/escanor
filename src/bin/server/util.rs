use serde_json::Value;



extern crate regex;
use regex::Regex;

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

pub fn is_numeric_with_regex(num_str: &String) -> bool {
    if num_str.is_empty() {
        return false;
    }
    let re = Regex::new(r"^(([1-9]*)|(([1-9]*)\.([0-9]*)))$").unwrap();
    re.is_match(num_str)
}

pub fn is_integer(num_str: &String) -> bool {
    if num_str.is_empty() {
        return false;
    }

    let first_char = num_str.chars().nth(0).unwrap();

    if !first_char.is_numeric() {
        return false;
    }

    if num_str.len() > 20 {
        return false;
    }
    let is_num = num_str.parse::<i64>().is_ok();
    return is_num;
}

pub fn is_json(json_str: &String) -> bool {
    let is_json = serde_json::from_slice::<Value>(json_str.as_bytes()).is_ok();
    return is_json;
}

/*
fn is_point(point_str : &String) -> bool {
    let p_str = point_str.replace("[","");

    let point_split = p_str.split(" ");

    return is_json;
}*/




mod tests {

    use super::*;

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
        "cities":[ "colombo" ]
    });

        merge(&mut a, &b);
        println!("{:#}", a);
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

