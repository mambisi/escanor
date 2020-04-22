use crate::command::*;
use crate::{error, util, unit_conv};
use serde_json::{Value};

use crate::db::ESValue;


pub fn analyse_token_stream(tokens: Vec<String>) -> Result<Box<dyn Command>, error::SyntaxError> {
    let empty_string: String = String::from("");
    let _default_type: String = String::from("string");
    let default_exp_time_str: String = String::from("0");

    let mut itr = tokens.iter();
    let cmd = itr.next().unwrap_or(&empty_string).to_lowercase();
    if cmd.eq("") {
        return Err(error::SyntaxError);
    }

    if cmd == "ping" {
        return Ok(Box::new(PingCmd));
    } else if cmd == "lastsave" {
        return Ok(Box::new(LastSaveCmd));
    }else if cmd == "dbsize" {
        return Ok(Box::new(DBSizeCmd));
    }
    else if cmd == "randomkey" {
        return Ok(Box::new(RandomKeyCmd));
    }
    else if cmd == "bgsave" {
        return Ok(Box::new(BGSaveCmd));
    } else if cmd == "flushdb" {
        return Ok(Box::new(FlushDBCmd));
    } else if cmd == "set" {
        let arg_key = itr.next().unwrap_or(&empty_string);
        if arg_key.is_empty() { return Err(error::SyntaxError); }

        let arg_value = itr.next().unwrap_or(&empty_string);
        if arg_value.is_empty() { return Err(error::SyntaxError); }

        let es_val = if util::is_integer(arg_value){
            let i = arg_value.parse::<i64>().unwrap();
            ESValue::Int(i)
        }else {
            ESValue::String(arg_value.to_owned())
        };

        let arg_ex_cmd = &itr.next().unwrap_or(&empty_string).to_lowercase();

        if arg_ex_cmd.is_empty() {
            return Ok(Box::new(SetCmd {
                arg_key: arg_key.to_owned(),
                arg_value: es_val,
                arg_exp: 0,
            }));
        } else if arg_ex_cmd == "ex" {
            let arg_next = itr.next().unwrap_or(&default_exp_time_str);
            let arg_exp = arg_next.parse::<u32>().unwrap_or(0);
            return Ok(Box::new(SetCmd {
                arg_key: arg_key.to_owned(),
                arg_value: es_val,
                arg_exp,
            }));
        }
    }else if cmd == "getset" {
        let arg_key = itr.next().unwrap_or(&empty_string);
        if arg_key.is_empty() { return Err(error::SyntaxError); }

        let arg_value = itr.next().unwrap_or(&empty_string);
        if arg_value.is_empty() { return Err(error::SyntaxError); }

        let es_val = if util::is_integer(arg_value){
            let i = arg_value.parse::<i64>().unwrap();
            ESValue::Int(i)
        }else {
            ESValue::String(arg_value.to_owned())
        };

        return Ok(Box::new(GetSetCmd {
            arg_key: arg_key.to_owned(),
            arg_value: es_val
        }));
    }else if cmd == "get" {
        let arg_key = itr.next().unwrap_or(&empty_string);
        if arg_key.is_empty() { return Err(error::SyntaxError); }
        return Ok(Box::new(GetCmd {
            arg_key: arg_key.to_owned()
        }));
    }else if cmd == "ttl" {
        let arg_key = itr.next().unwrap_or(&empty_string);
        if arg_key.is_empty() { return Err(error::SyntaxError); }
        return Ok(Box::new(TTLCmd {
            arg_key: arg_key.to_owned()
        }));
    }else if cmd == "persist" {
        let arg_key = itr.next().unwrap_or(&empty_string);
        if arg_key.is_empty() { return Err(error::SyntaxError); }
        return Ok(Box::new(PersistCmd {
            arg_key: arg_key.to_owned()
        }));
    }else if cmd == "expire" {
        let arg_key = itr.next().unwrap_or(&empty_string);
        if arg_key.is_empty() { return Err(error::SyntaxError); }

        let arg_value = itr.next().unwrap_or(&empty_string);
        if arg_value.is_empty() { return Err(error::SyntaxError); }

        return match arg_value.parse::<i64>() {
            Ok(t) => {
                if t < 0 {
                    return Err(error::SyntaxError);
                }
                Ok(Box::new(ExpireCmd {
                    arg_key: arg_key.to_owned(),
                    arg_value : t
                }))
            },
            Err(_) => {
                Err(error::SyntaxError)
            },
        }
    }
    else if cmd == "expire_at" {
        let arg_key = itr.next().unwrap_or(&empty_string);
        if arg_key.is_empty() { return Err(error::SyntaxError); }

        let arg_value = itr.next().unwrap_or(&empty_string);
        if arg_value.is_empty() { return Err(error::SyntaxError); }

        return match arg_value.parse::<i64>() {
            Ok(t) => {
                if t < 0 {
                    return Err(error::SyntaxError);
                }
                Ok(Box::new(ExpireAtCmd {
                    arg_key: arg_key.to_owned(),
                    arg_value : t
                }))
            },
            Err(_) => {
                Err(error::SyntaxError)
            },
        }
    }
    else if cmd == "del" {
        let arg_key = itr.next().unwrap_or(&empty_string);
        if arg_key.is_empty() { return Err(error::SyntaxError); }
        return Ok(Box::new(DelCmd {
            arg_key: arg_key.to_owned()
        }));
    } else if cmd == "keys" {
        let arg_pattern = itr.next().unwrap_or(&empty_string);
        if arg_pattern.is_empty() { return Err(error::SyntaxError); }
        return Ok(Box::new(KeysCmd {
            pattern: arg_pattern.to_owned()
        }));
    } else if cmd == "exists" {
        let mut keys: Vec<String> = vec![];

        while let Some(i) = itr.next() {
            keys.push(i.to_owned());
        }
        if keys.is_empty() {
            return Err(error::SyntaxError);
        }

        return Ok(Box::new(ExistsCmd {
            keys
        }));
    } else if cmd == "info" {
        return Ok(Box::new(InfoCmd));
    }
    // GEOADD [key] long lat tag [long lat tag...]
    else if cmd == "geoadd" {
        let arg_key = itr.next().unwrap_or(&empty_string);
        if arg_key.is_empty() { return Err(error::SyntaxError); }
        let mut items_after_key: Vec<&String> = vec![];

        while let Some(i) = itr.next() {
            items_after_key.push(i);
        }
        if items_after_key.is_empty() {
            return Err(error::SyntaxError);
        }
        if items_after_key.len() % 3 != 0 {
            return Err(error::SyntaxError);
        }
        //split items_after_key in arrays of [3, &String]
        let mut geo_point_chunks = items_after_key.chunks_exact(3);

        let mut items: Vec<CmdGeoItem> = vec![];

        while let Some(c) = geo_point_chunks.next() {
            let lng = c[0];
            let lat = c[1];
            let tag = c[2];

            if !(util::is_numeric(lat) && util::is_numeric(lng)) {
                return Err(error::SyntaxError);
            }

            let lat = lat.parse::<f64>().unwrap();
            let lng = lng.parse::<f64>().unwrap();
            let tag = tag.to_owned();

            items.push((lat, lng, tag))
        }

        return Ok(Box::new(GeoAddCmd {
            arg_key: arg_key.to_owned(),
            items,
        }));
    } else if cmd == "geojson" {
        let arg_key = itr.next().unwrap_or(&empty_string);
        if arg_key.is_empty() { return Err(error::SyntaxError); }
        let mut items_after_key: Vec<String> = vec![];

        while let Some(i) = itr.next() {
            items_after_key.push(i.to_owned());
        }

        if items_after_key.is_empty() {
            return Err(error::SyntaxError);
        }

        return Ok(Box::new(GeoJsonCmd {
            arg_key: arg_key.to_owned(),
            items: items_after_key,
        }));
    } else if cmd == "geohash" {
        let arg_key = itr.next().unwrap_or(&empty_string);
        if arg_key.is_empty() { return Err(error::SyntaxError); }
        let mut items_after_key: Vec<String> = vec![];

        while let Some(i) = itr.next() {
            items_after_key.push(i.to_owned());
        }

        if items_after_key.is_empty() {
            return Err(error::SyntaxError);
        }

        return Ok(Box::new(GeoHashCmd {
            arg_key: arg_key.to_owned(),
            items: items_after_key,
        }));
    } else if cmd == "geopos" {
        let arg_key = itr.next().unwrap_or(&empty_string);
        if arg_key.is_empty() { return Err(error::SyntaxError); }
        let mut items_after_key: Vec<String> = vec![];

        while let Some(i) = itr.next() {
            items_after_key.push(i.to_owned());
        }

        if items_after_key.is_empty() {
            return Err(error::SyntaxError);
        }

        return Ok(Box::new(GeoPosCmd {
            arg_key: arg_key.to_owned(),
            items: items_after_key,
        }));
    } else if cmd == "georadius" {
        let arg_key = itr.next().unwrap_or(&empty_string);
        if arg_key.is_empty() { return Err(error::SyntaxError); }

        let arg_lng = itr.next().unwrap_or(&empty_string);
        if arg_lng.is_empty() { return Err(error::SyntaxError); }

        let arg_lat = itr.next().unwrap_or(&empty_string);
        if arg_lat.is_empty() { return Err(error::SyntaxError); }

        let arg_radius = itr.next().unwrap_or(&empty_string);
        if arg_radius.is_empty() { return Err(error::SyntaxError); }

        let arg_unit_string = &itr.next().unwrap_or(&empty_string).to_lowercase();
        if arg_unit_string.is_empty() { return Err(error::SyntaxError); }

        let arg_unit = match unit_conv::parse(arg_unit_string) {
            Ok(unit) => unit,
            Err(_e) => {
                return Err(error::SyntaxError);
            }
        };

        let arg_order_string = itr.next().unwrap_or(&empty_string).to_lowercase();
        let mut arg_order = ArgOrder::UNSPECIFIED;

        match check_validate_arg_order(arg_order_string, &mut arg_order) {
            Ok(()) => (),
            Err(e) => {
                return Err(e);
            }
        };

        if !(util::is_numeric(arg_lng) && util::is_numeric(arg_lng) && util::is_numeric(arg_radius)) {
            return Err(error::SyntaxError);
        }

        let lat = arg_lat.parse::<f64>().unwrap();
        let lng = arg_lng.parse::<f64>().unwrap();
        let rads = arg_radius.parse::<f64>().unwrap();

        return Ok(Box::new(GeoRadiusCmd {
            arg_key: arg_key.to_owned(),
            arg_lng: lng,
            arg_lat: lat,
            arg_radius: rads,
            arg_unit,
            arg_order,
        }));
    } else if cmd == "geodist" {
        let arg_key = itr.next().unwrap_or(&empty_string);
        if arg_key.is_empty() { return Err(error::SyntaxError); }

        let member_1 = itr.next().unwrap_or(&empty_string);
        if member_1.is_empty() { return Err(error::SyntaxError); }

        let member_2 = itr.next().unwrap_or(&empty_string);
        if member_2.is_empty() { return Err(error::SyntaxError); }

        let arg_unit_string = &itr.next().unwrap_or(&empty_string).to_lowercase();
        if arg_unit_string.is_empty() { return Err(error::SyntaxError); }

        let arg_unit = match unit_conv::parse(arg_unit_string) {
            Ok(unit) => unit,
            Err(_e) => {
                return Err(error::SyntaxError);
            }
        };

        return Ok(Box::new(GeoDistCmd {
            arg_key: arg_key.to_owned(),
            arg_mem_1: member_1.to_owned(),
            arg_mem_2: member_2.to_owned(),
            arg_unit,
        }));
    } else if cmd == "georadiusbymember" {
        let arg_key = itr.next().unwrap_or(&empty_string);
        if arg_key.is_empty() { return Err(error::SyntaxError); }

        let arg_member = itr.next().unwrap_or(&empty_string);
        if arg_member.is_empty() { return Err(error::SyntaxError); }

        let arg_radius = itr.next().unwrap_or(&empty_string);
        if arg_radius.is_empty() { return Err(error::SyntaxError); }

        let arg_unit_string = &itr.next().unwrap_or(&empty_string).to_lowercase();
        if arg_unit_string.is_empty() { return Err(error::SyntaxError); }

        let arg_unit = match unit_conv::parse(arg_unit_string) {
            Ok(unit) => unit,
            Err(_e) => {
                return Err(error::SyntaxError);
            }
        };

        let arg_order_string = itr.next().unwrap_or(&empty_string).to_lowercase();
        let mut arg_order = ArgOrder::UNSPECIFIED;

        match check_validate_arg_order(arg_order_string, &mut arg_order) {
            Ok(()) => (),
            Err(e) => {
                return Err(e);
            }
        };

        if !(util::is_numeric(arg_radius)) {
            return Err(error::SyntaxError);
        }
        let rads = arg_radius.parse::<f64>().unwrap();

        return Ok(Box::new(
            GeoRadiusByMemberCmd {
                arg_key: arg_key.to_owned(),
                member: arg_member.to_string(),
                arg_radius: rads,
                arg_unit,
                arg_order,
            }
        ));
    } else if cmd == "geodel" {
        let arg_key = itr.next().unwrap_or(&empty_string);
        if arg_key.is_empty() { return Err(error::SyntaxError); }
        return Ok(Box::new(GeoDelCmd {
            arg_key: arg_key.to_owned()
        }));
    } else if cmd == "georem" {
        let arg_key = itr.next().unwrap_or(&empty_string);
        if arg_key.is_empty() { return Err(error::SyntaxError); }
        let mut items_after_key: Vec<String> = vec![];

        while let Some(i) = itr.next() {
            items_after_key.push(i.to_owned());
        }

        if items_after_key.is_empty() {
            return Err(error::SyntaxError);
        }

        return Ok(Box::new(GeoRemoveCmd {
            arg_key: arg_key.to_owned(),
            items: items_after_key,
        }));
    } else if cmd == "jsetr" {
        let arg_key = itr.next().unwrap_or(&empty_string);
        if arg_key.is_empty() { return Err(error::SyntaxError); }

        let arg_value = itr.next().unwrap_or(&empty_string);
        if arg_value.is_empty() { return Err(error::SyntaxError); }
        return Ok(Box::new(JSetRawCmd {
            arg_key: arg_key.to_owned(),
            arg_value: arg_value.to_owned(),
        }));
    } else if cmd == "jset" {
        let arg_key = itr.next().unwrap_or(&empty_string);
        if arg_key.is_empty() { return Err(error::SyntaxError); }

        let mut items_after_key: Vec<&String> = vec![];

        while let Some(i) = itr.next() {
            items_after_key.push(i);
        }
        if items_after_key.is_empty() {
            return Err(error::SyntaxError);
        }
        if items_after_key.len() % 2 != 0 {
            return Err(error::SyntaxError);
        }
        //split items_after_key in arrays of [3, &String]
        let mut geo_point_chunks = items_after_key.chunks_exact(2);

        let mut items: Vec<JSetArgItem> = vec![];

        while let Some(c) = geo_point_chunks.next() {
            let dot_path = c[0];
            let value_string = c[1];

            if util::is_numeric(value_string) {
                let v = value_string.parse::<f64>().unwrap();
                if v.fract() == 0.0 {
                    let vs = v as i64;
                    items.push((dot_path.to_owned(), json!(vs)));
                } else {
                    items.push((dot_path.to_owned(), json!(v)));
                }
            } else {
                items.push((dot_path.to_owned(), Value::String(value_string.to_owned())));
            }
        }
        return Ok(Box::new(JSetCmd {
            arg_key: arg_key.to_owned(),
            arg_set_items: items,
        }));
    } else if cmd == "jmerge" {
        let arg_key = itr.next().unwrap_or(&empty_string);
        if arg_key.is_empty() { return Err(error::SyntaxError); }

        let arg_value = itr.next().unwrap_or(&empty_string);
        if arg_value.is_empty() { return Err(error::SyntaxError); }
        return Ok(Box::new(JMergeCmd {
            arg_key: arg_key.to_owned(),
            arg_value: arg_value.to_owned(),
        }));
    } else if cmd == "jget" {
        let arg_key = itr.next().unwrap_or(&empty_string);
        if arg_key.is_empty() { return Err(error::SyntaxError); }
        let arg_value = itr.next().unwrap_or(&empty_string);

        return Ok(Box::new(JGetCmd {
            arg_key: arg_key.to_owned(),
            arg_dot_path: if arg_value.is_empty() { None } else { Some(arg_value.to_owned()) },
        }));
    } else if cmd == "jpath" {
        let arg_key = itr.next().unwrap_or(&empty_string);
        if arg_key.is_empty() { return Err(error::SyntaxError); }

        let arg_selector = itr.next().unwrap_or(&empty_string);
        if arg_selector.is_empty() { return Err(error::SyntaxError); }
        return Ok(Box::new(JPathCmd {
            arg_key: arg_key.to_owned(),
            arg_selector: arg_selector.to_owned(),
        }));
    } else if cmd == "jdel" {
        let arg_key = itr.next().unwrap_or(&empty_string);
        if arg_key.is_empty() { return Err(error::SyntaxError); }

        return Ok(Box::new(JDelCmd {
            arg_key: arg_key.to_owned(),
        }));
    } else if cmd == "jincrby" {
        let arg_key = itr.next().unwrap_or(&empty_string);
        if arg_key.is_empty() { return Err(error::SyntaxError); }

        let arg_path = itr.next().unwrap_or(&empty_string);
        if arg_path.is_empty() { return Err(error::SyntaxError); }

        let arg_value = itr.next().unwrap_or(&empty_string);
        if arg_value.is_empty() { return Err(error::SyntaxError); }

        if !util::is_integer(arg_value) {
            return Err(error::SyntaxError);
        }

        let incr_value = arg_value.parse::<i64>().unwrap_or(0);

        return Ok(Box::new(JIncrByCmd {
            arg_key: arg_key.to_owned(),
            arg_path: arg_path.to_owned(),
            arg_increment_value: incr_value,
        }));
    } else if cmd == "jincrbyfloat" {
        let arg_key = itr.next().unwrap_or(&empty_string);
        if arg_key.is_empty() { return Err(error::SyntaxError); }

        let arg_path = itr.next().unwrap_or(&empty_string);
        if arg_path.is_empty() { return Err(error::SyntaxError); }

        let arg_value = itr.next().unwrap_or(&empty_string);
        if arg_value.is_empty() { return Err(error::SyntaxError); }

        if !util::is_numeric(arg_value) {
            return Err(error::SyntaxError);
        }

        let incr_value = arg_value.parse::<f64>().unwrap_or(0.0);
        if incr_value == 0.0 {
            return Err(error::SyntaxError);
        }

        return Ok(Box::new(JIncrByFloatCmd {
            arg_key: arg_key.to_owned(),
            arg_path: arg_path.to_owned(),
            arg_increment_value: incr_value,
        }));
    }

    Err(error::SyntaxError)
}

fn check_validate_arg_order(arg_order_string: String, arg_order: &mut ArgOrder) -> Result<(), error::SyntaxError> {
    if arg_order_string.is_empty() {
        return Ok(());
    } else if !arg_order_string.is_empty() && (arg_order_string == "asc" || arg_order_string == "desc") {
        *arg_order = match arg_order_string.as_str() {
            "asc" => ArgOrder::ASC,
            "desc" => ArgOrder::DESC,
            _ => {
                return Err(error::SyntaxError);
            }
        };
        return Ok(());
    }
    Err(error::SyntaxError)
}

