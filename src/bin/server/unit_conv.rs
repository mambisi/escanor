#[derive(Debug,Clone,Copy)]
pub enum Units {
    Miles,
    Kilometers,
    Meters,
}

pub fn parse(s: &String) -> Result<Units, ()> {
    match s.as_str() {
        "km" => Ok(Units::Kilometers),
        "m" => Ok(Units::Meters),
        "mi" => Ok(Units::Miles),
        _ => Err(()),
    }
}


pub fn m_km(m: f64) -> f64 { m / 1000.000 }

pub fn m_mi(m: f64) -> f64 { m / 1609.344 }

pub fn mi_m(mi: f64) -> f64 { mi * 1609.344 }

pub fn km_m(km: f64) -> f64 { km * 1000.000 }

