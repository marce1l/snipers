use crate::{
    telegram::bot::WATCHED_WALLETS,
    utils::{hex_to_decimal, to_eth, to_gwei},
};
use chrono::{DateTime, Datelike};
use std::{collections::HashMap, sync::Arc, thread};
use tokio::sync::Mutex;

mod alchemy;
mod etherscan;
mod honeypot;

use alchemy::{AlchemyAPI, TokenBalance, TokenBalancesResult};
use etherscan::{
    EtherscanAPI, EtherscanEthPrices, EtherscanNormalTransaction, EtherscanTokenTransaction,
};

pub async fn get_eth_price() -> Result<f64, reqwest::Error> {
    match EtherscanAPI::<EtherscanEthPrices>::eth_price().await {
        Ok(response) => Ok(response.result.ethusd.parse::<f64>().unwrap()),
        Err(e) => Err(e.without_url()),
    }
}

pub async fn get_normal_transactions(
    address: String,
) -> Result<Vec<EtherscanNormalTransaction>, reqwest::Error> {
    match EtherscanAPI::<Vec<EtherscanNormalTransaction>>::get_normal_transactions(address).await {
        Ok(response) => Ok(response.result),
        Err(e) => Err(e.without_url()),
    }
}

pub async fn get_token_transactions(
    address: String,
) -> Result<Vec<EtherscanTokenTransaction>, reqwest::Error> {
    match EtherscanAPI::<Vec<EtherscanTokenTransaction>>::get_token_transactions(address).await {
        Ok(response) => Ok(response.result),
        Err(e) => Err(e.without_url()),
    }
}

pub async fn get_eth_gas() -> Result<f64, reqwest::Error> {
    match AlchemyAPI::<String>::get_eth_gas().await {
        Ok(gas) => Ok(to_gwei(&gas.result)),
        Err(e) => Err(e.without_url()),
    }
}

pub async fn get_eth_balance() -> Result<String, reqwest::Error> {
    match AlchemyAPI::<String>::get_eth_balance().await {
        Ok(balance) => Ok(format!("{}", to_eth(&balance.result))),
        Err(e) => Err(e.without_url()),
    }
}

pub async fn get_token_balances() -> Result<HashMap<String, HashMap<String, String>>, reqwest::Error>
{
    match AlchemyAPI::<TokenBalancesResult>::get_token_balances().await {
        Ok(token_balances) => Ok(to_owned_tokens(token_balances.result.token_balances).await),
        Err(e) => Err(e.without_url()),
    }
}

async fn to_owned_tokens(
    token_balances: Vec<TokenBalance>,
) -> HashMap<String, HashMap<String, String>> {
    let mut tokens = HashMap::new();

    for tb in token_balances {
        if tb.token_balance != "0x0000000000000000000000000000000000000000000000000000000000000000"
        {
            match honeypot::get_token_info(tb.contract_address.clone()).await {
                Ok(token_info) => {
                    tokens.insert(
                        token_info.name,
                        HashMap::from([
                            (String::from("contract"), tb.contract_address),
                            (String::from("symbol"), token_info.symbol),
                            (
                                String::from("balance"),
                                format!(
                                    "{:.2}",
                                    hex_to_decimal(&tb.token_balance) as f64
                                        / 10.0f64.powf(token_info.decimals as f64)
                                ),
                            ),
                            // TODO: need another api for fetching a token current price
                            (String::from("balance_usd"), format!("{}", to_eth("0x0000"))),
                        ]),
                    );
                }
                Err(_) => {
                    tokens.insert(
                        tb.contract_address.clone(),
                        HashMap::from([
                            (String::from("contract"), tb.contract_address),
                            (String::from("symbol"), String::from("")),
                            (String::from("balance"), String::from("")),
                            (String::from("balance_usd"), String::from("")),
                        ]),
                    );
                }
            }
        }
    }

    tokens
}

pub async fn watch_wallets() {
    let mut last_transcations = HashMap::<String, u64>::new();

    loop {
        thread::sleep(chrono::Duration::try_seconds(60).unwrap().to_std().unwrap());
        let watched_wallets = WATCHED_WALLETS.lock().await;

        if watched_wallets.is_empty() {
            continue;
        };

        if last_transcations.is_empty() {
            last_transcations = get_latest_token_transactions(watched_wallets.to_vec()).await;
        } else {
            let new_transactions =
                check_for_new_token_transactions(watched_wallets.to_vec(), &last_transcations)
                    .await;

            for (wallet, transactions) in new_transactions {
                match &transactions {
                    Some(val) => {
                        // replace latest transaction block number
                        last_transcations
                            .insert(wallet, val[0].block_number.trim().parse::<u64>().unwrap());

                        for v in val {
                            // TODO: send telegram notification
                        }
                    }
                    None => {}
                };
            }
        }
    }
}

async fn get_latest_token_transactions(wallets: Vec<String>) -> HashMap<String, u64> {
    let mut token_transactions = HashMap::<String, u64>::new();

    for w in wallets {
        match get_token_transactions(w.to_owned()).await {
            Ok(val) => {
                token_transactions.insert(w, val[0].block_number.trim().parse::<u64>().unwrap());
                continue;
            }
            Err(_) => continue,
        };
    }

    token_transactions
}

async fn check_for_new_token_transactions(
    wallets: Vec<String>,
    last_transactions: &HashMap<String, u64>,
) -> HashMap<String, Option<Vec<EtherscanTokenTransaction>>> {
    let mut token_transactions = HashMap::<String, Option<Vec<EtherscanTokenTransaction>>>::new();

    for w in wallets {
        match get_token_transactions(w.to_owned()).await {
            Ok(val) => {
                let block_number = last_transactions.get(&w).unwrap();
                let mut transactions = Vec::<EtherscanTokenTransaction>::new();

                for i in 0..val.len() {
                    if &val[i].block_number.trim().parse::<u64>().unwrap() > block_number {
                        transactions.push(val[i].clone());
                    } else {
                        if i == 0 {
                            token_transactions.insert(w, None);
                        } else {
                            token_transactions.insert(w, Some(transactions));
                        }

                        break;
                    }
                }
            }
            Err(_) => continue,
        };
    }

    token_transactions
}

/*

TODO:
    - Handle mutex poisoning
        https://blog.logrocket.com/understanding-handling-rust-mutex-poisoning/
    - Handle possible thread panicking

*/

pub fn start_cu_instance() -> CU {
    let mut compute_unit: CU = CU::default();
    compute_unit.start();

    compute_unit
}

impl CUInner {
    fn default() -> Self {
        Self {
            used_cu: Mutex::new(0),
            max_cu: 300_000_000,
            days_since_reset: Mutex::new(0),
        }
    }

    async fn add_cu(&self, cu: u32) {
        *self.used_cu.lock().await += cu;
    }

    async fn start_of_month_reset_cu(&self) {
        let utc_date: DateTime<chrono::Utc> = chrono::Utc::now();
        let mut days_since_reset = self.days_since_reset.lock().await;

        if utc_date.day() == 1 || (*days_since_reset >= 28 && utc_date.day() == 2) {
            let mut used_cu = self.used_cu.lock().await;
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

    fn start(&mut self) {
        let local_self = self.inner.clone();

        tokio::spawn(async move {
            loop {
                thread::sleep(chrono::Duration::try_days(1).unwrap().to_std().unwrap());

                local_self.start_of_month_reset_cu().await;
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
