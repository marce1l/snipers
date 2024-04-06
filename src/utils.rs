pub fn hex_to_decimal(hex: &String) -> u128 {
    let rm_prefix = hex.trim_start_matches("0x");
    u128::from_str_radix(rm_prefix, 16).unwrap()
}

pub fn to_eth(hex: &String) -> f64 {
    let wei = hex_to_decimal(&hex);
    let eth: f64 = wei as f64 / 10.0f64.powf(18.0);
    eth
}

pub fn to_gwei(hex: &String) -> f64 {
    let wei = hex_to_decimal(&hex);
    let gwei: f64 = wei as f64 / 10.0f64.powf(9.0);
    gwei
}
