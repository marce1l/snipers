use chrono::{ DateTime, Datelike };
use std::{env, fmt, sync::{Arc, Mutex}, thread};
use reqwest::{header::CONTENT_TYPE, Client};
use serde::{de, Deserialize, Serialize};
use serde_json;

/*

TODO:
    - Handle mutex poisoning
        https://blog.logrocket.com/understanding-handling-rust-mutex-poisoning/
    - Handle possible thread panicking

*/


impl CUInner {
    fn default() -> Self {
        Self {
            used_cu: Mutex::new(0),
            max_cu: 300_000_000,
            days_since_reset: Mutex::new(0),
        }
    }

    fn add_cu(&self, cu: u32) {
        *self.used_cu.lock().unwrap() += cu;
    }

    fn start_of_month_reset_cu(&self) {
        let utc_date: DateTime<chrono::Utc> = chrono::Utc::now();
        let mut days_since_reset = self.days_since_reset.lock().unwrap();

        if utc_date.day() == 1 || ( *days_since_reset >= 28 && utc_date.day() == 2 ) {
            let mut used_cu = self.used_cu.lock().unwrap();
            *used_cu = 0;
            *days_since_reset = 0;
        } else {
            *days_since_reset += 1;
        };
    }


}

impl CU {
    fn default() -> Self {
        Self {
            inner: Arc::new(CUInner::default()),
        }
    }

    // calls the 'start_of_month_reset_cu' function once a day
    fn start(&mut self) {
        let local_self = self.inner.clone();

        thread::spawn(move || {
            loop {
                thread::sleep(chrono::Duration::try_days(1).unwrap().to_std().unwrap());

                local_self.start_of_month_reset_cu();
            }
        });
    }
}

#[derive(Debug, Default)]
struct CUInner {
    used_cu: Mutex<u32>,
    max_cu: u32,
    days_since_reset: Mutex<u8>,
}

#[derive(Debug, Default)]
pub struct CU {
    inner: Arc<CUInner>,
}

pub fn start_cu_instance() -> CU {
    let mut compute_unit: CU = CU::default();
    compute_unit.start();

    compute_unit
}


pub async fn get_gas() -> f64 {
    tokio::task::spawn_blocking(|| {
        let gas = AlchemyAPI::<String>::get_eth_gas();
        to_gwei(&gas.unwrap().result)
    }).await.expect("AlchemyAPI 'get_gas' method panicked")
}

pub async fn get_eth_balance() -> String {
    tokio::task::spawn_blocking(|| {
        let balance = AlchemyAPI::<String>::get_eth_balance(env::var("ETH_ADDRESS").unwrap());
        format!("{} eth", to_eth(&balance.unwrap().result))
    }).await.expect("AlchemyAPI 'get_balance' method panicked")
}

pub async fn get_token_balances() -> Vec<TokenBalances> {
    tokio::task::spawn_blocking(|| {
        let token_balances = AlchemyAPI::<TokenBalancesResult>::get_token_balances(env::var("ETH_ADDRESS").unwrap());
        token_balances.unwrap().result.token_balances
    }).await.expect("AlchemyAPI 'get_token_balances' method panicked")
}

fn to_eth(hex: &String) -> f64 {
    let rm_prefix = hex.trim_start_matches("0x");
    let wei = u64::from_str_radix(rm_prefix, 16);
    let eth: f64 = wei.unwrap() as f64 / 10.0f64.powf(18.0);
    eth
}

fn to_gwei(hex: &String) -> f64 {
    let rm_prefix = hex.trim_start_matches("0x");
    let wei = u64::from_str_radix(rm_prefix, 16);
    let gwei: f64 = wei.unwrap() as f64 / 10.0f64.powf(9.0);
    gwei
}

impl<T: de::DeserializeOwned> AlchemyAPI<T> {

    // function currently has to consume body arg as it would have to have a 'static lifetime.
    #[tokio::main]
    async fn send_request(payload: AlchemyPayload) -> Result<AlchemyAPI<T>, reqwest::Error> {

        let response = Client::new()
            .post(format!("https://eth-mainnet.g.alchemy.com/v2/{}", env::var("ALCHEMY_API").unwrap()))
            .header(CONTENT_TYPE, "applciation/json")
            .body(serde_json::to_string(&payload).unwrap())
            .send()
            .await
            .expect("failed response")
            .json()
            .await?;

        Ok(response)
    }

    fn get_eth_balance(address: String) -> Result<AlchemyAPI<String>, reqwest::Error> {
        let payload: AlchemyPayload = AlchemyPayload {
            params: Some(vec![
                String::from(address),
                String::from("latest"),
            ]),
            method: String::from("eth_getBalance"),
            ..AlchemyPayload::default()
        };

        AlchemyAPI::send_request(payload)
    }

    fn get_eth_gas() -> Result<AlchemyAPI<String>, reqwest::Error> {
        let payload: AlchemyPayload = AlchemyPayload {
            method: String::from("eth_gasPrice"),
            ..AlchemyPayload::default()
        };

        AlchemyAPI::send_request(payload)
    }

    fn get_token_balances(address: String) -> Result<AlchemyAPI<TokenBalancesResult>, reqwest::Error> {
        let payload: AlchemyPayload = AlchemyPayload {
            params: Some(vec![
                String::from(address)
            ]),
            method: String::from("alchemy_getTokenBalances"),
            ..AlchemyPayload::default()
        };

        AlchemyAPI::send_request(payload)
    }

}

#[derive(Debug, Deserialize)]
struct AlchemyAPI<T> {
    jsonrpc: String,
    id: u32,
    result: T,
}

impl AlchemyPayload {
    fn default() -> Self {
        Self {
            id: 1,
            jsonrpc: String::from("2.0"),
            params: None,
            ..Default::default()
        }
    }
}

#[derive(Debug, Default, Serialize)]
struct AlchemyPayload {
    id: u8,
    jsonrpc: String,
    params: Option<Vec<String>>,
    method: String
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TokenBalancesResult {
    address: String,
    token_balances: Vec<TokenBalances>
}

impl fmt::Display for TokenBalances {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "contract: {}\nbalance: {}", self.contract_address, self.token_balance)
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenBalances {
    contract_address: String,
    token_balance: String
}