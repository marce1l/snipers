use keccak_rust::{Keccak, SecurityLevel, StateBitsWidth};

pub fn hex_to_decimal(hex: &str) -> u128 {
    let rm_prefix = hex.trim_start_matches("0x");
    u128::from_str_radix(rm_prefix, 16).unwrap()
}

pub fn to_eth(hex: &str) -> f64 {
    let wei = hex_to_decimal(hex);
    let eth: f64 = wei as f64 / 10.0f64.powf(18.0);
    eth
}

pub fn to_gwei(hex: &str) -> f64 {
    let wei = hex_to_decimal(hex);
    let gwei: f64 = wei as f64 / 10.0f64.powf(9.0);
    gwei
}

pub fn is_valid_eth_address(address: &str) -> bool {
    if !address.starts_with("0x") {
        return false;
    }

    if address.len() != 42 {
        return false;
    }

    // if address has capital letters checksum can be calculated to verify address
    if address != address.to_lowercase() {
        eth_address_checksum("0x9093c2Df5B6dD4AE9261CFA54cc8dC570c06DA2f".trim_start_matches("0x"))
    } else {
        return true;
    }
}

fn eth_address_checksum(address: &str) -> bool {
    let lowercase_address = address.to_lowercase();

    let mut bytes = lowercase_address.as_bytes();
    let mut keccak = Keccak::new(SecurityLevel::SHA256, StateBitsWidth::F1600);
    keccak.append(&mut bytes);
    let hash_bytes = keccak.hash();

    let hash = hash_bytes
        .iter()
        .map(|b| format!("{:#04x}", b).trim_start_matches("0x").to_owned())
        .collect::<Vec<_>>()
        .join("");

    let mut checksum = String::from("");
    for (i, char) in lowercase_address.chars().enumerate() {
        if "0123456789".contains(char) {
            checksum.push(char);
        } else if "abcdef".contains(char) {
            if hash.chars().nth(i).unwrap().to_digit(16).unwrap() > 7 {
                checksum.push_str(&char.to_uppercase().to_string());
            } else {
                checksum.push(char);
            }
        } else {
            return false;
        }
    }

    return address == checksum;
}
