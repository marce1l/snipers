use crate::{
    telegram::bot::{self, WATCHED_WALLETS},
    utils::{hex_to_decimal, to_eth, to_gwei},
};
use chrono::{DateTime, Datelike};
use std::{collections::HashMap, sync::Arc, thread};
use teloxide::{types::ChatId, Bot};
use tokio::sync::Mutex;

mod alchemy;
mod etherscan;
mod honeypot;
mod moralis;

use alchemy::{AlchemyAPI, TokenBalance, TokenBalancesResult};
pub use etherscan::EtherscanTokenTransaction;
use etherscan::{EtherscanAPI, EtherscanEthPrices, EtherscanNormalTransaction};
pub use honeypot::TokenInfo;
pub use moralis::TokenOwnersResult;

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

pub async fn get_top_token_holders(
    contract: String,
) -> Result<Vec<TokenOwnersResult>, reqwest::Error> {
    match moralis::get_top_token_holders(contract).await {
        Ok(token_owners) => Ok(token_owners.result),
        Err(e) => Err(e.without_url()),
    }
}

pub async fn get_token_price(contract: String) -> Result<f32, reqwest::Error> {
    match moralis::get_token_price(contract).await {
        Ok(price) => Ok(price.usd_price),
        Err(e) => Err(e.without_url()),
    }
}

pub async fn get_token_info(contract: String) -> Result<TokenInfo, reqwest::Error> {
    match honeypot::get_token_info(contract).await {
        Ok(token_info) => Ok(token_info),
        Err(e) => Err(e.without_url()),
    }
}

async fn to_owned_tokens(
    token_balances: Vec<TokenBalance>,
) -> HashMap<String, HashMap<String, String>> {
    let mut tokens = HashMap::new();

    for tb in token_balances {
        if tb.token_balance.parse::<f64>().unwrap_or(0.0) != 0.0 {
            match honeypot::get_token_info(tb.contract_address.clone()).await {
                Ok(token_info) => {
                    let token_price = match get_token_price(tb.contract_address.clone()).await {
                        Ok(price) => price,
                        Err(_) => 0.0,
                    };

                    let balance = hex_to_decimal(&tb.token_balance) as f64
                        / 10.0f64.powf(token_info.decimals as f64);

                    tokens.insert(
                        token_info.name,
                        HashMap::from([
                            (String::from("contract"), tb.contract_address),
                            (String::from("symbol"), token_info.symbol),
                            (String::from("balance"), format!("{:.2}", balance)),
                            (
                                String::from("balance_usd"),
                                format!("{:.2}", token_price as f64 * balance),
                            ),
                        ]),
                    );
                }
                Err(_) => continue,
            }
        }
    }

    tokens
}

pub async fn watch_wallets(bot: Bot) {
    let mut last_transcations = HashMap::<ChatId, HashMap<String, u64>>::new();

    loop {
        thread::sleep(chrono::Duration::try_seconds(10).unwrap().to_std().unwrap());
        let watched_wallets = WATCHED_WALLETS.lock().await;

        if watched_wallets.is_empty() {
            continue;
        };

        if last_transcations.is_empty() {
            last_transcations = get_latest_token_transactions(watched_wallets.to_owned()).await;
        } else {
            let new_transactions_by_chat_id =
                check_for_new_token_transactions(watched_wallets.to_owned(), &last_transcations)
                    .await;

            for (chat_id, new_transactions) in new_transactions_by_chat_id {
                for (wallet, transactions) in new_transactions {
                    match &transactions {
                        Some(val) => {
                            // replace latest transaction timestamp
                            last_transcations
                                .entry(chat_id)
                                .and_modify(|map| {
                                    map.insert(
                                        wallet.clone(),
                                        val[0].time_stamp.trim().parse::<u64>().unwrap_or(0),
                                    );
                                    ()
                                })
                                .or_insert_with(|| {
                                    HashMap::from([(
                                        wallet.clone(),
                                        val[0].time_stamp.trim().parse::<u64>().unwrap_or(0),
                                    )])
                                });

                            for v in val.iter().rev() {
                                let _ = bot::watched_wallet_notification(
                                    bot.clone(),
                                    chat_id,
                                    wallet.clone(),
                                    v,
                                )
                                .await;
                            }
                        }
                        None => continue,
                    };
                }
            }
        }
    }
}

async fn get_latest_token_transactions(
    watched_wallets: HashMap<ChatId, Vec<String>>,
) -> HashMap<ChatId, HashMap<String, u64>> {
    let mut token_transactions = HashMap::<ChatId, HashMap<String, u64>>::new();

    for (chat_id, wallets) in watched_wallets {
        for w in wallets {
            match get_token_transactions(w.to_owned()).await {
                Ok(val) => {
                    token_transactions
                        .entry(chat_id)
                        .and_modify(|map| {
                            map.insert(
                                w.to_owned(),
                                val[0].time_stamp.trim().parse::<u64>().unwrap_or(0),
                            );
                            ()
                        })
                        .or_insert_with(|| {
                            HashMap::from([(
                                w,
                                val[0].time_stamp.trim().parse::<u64>().unwrap_or(0),
                            )])
                        });
                    continue;
                }
                Err(_) => {
                    continue;
                }
            };
        }
    }

    token_transactions
}

async fn check_for_new_token_transactions(
    watched_wallets: HashMap<ChatId, Vec<String>>,
    last_transactions: &HashMap<ChatId, HashMap<String, u64>>,
) -> HashMap<ChatId, HashMap<String, Option<Vec<EtherscanTokenTransaction>>>> {
    let mut token_transactions =
        HashMap::<ChatId, HashMap<String, Option<Vec<EtherscanTokenTransaction>>>>::new();

    for (chat_id, wallets) in watched_wallets {
        for w in wallets {
            match get_token_transactions(w.to_owned()).await {
                Ok(val) => {
                    let time_stamp = last_transactions
                        .get(&chat_id)
                        .unwrap()
                        .get(&w)
                        .unwrap_or(&0);
                    let mut transactions = Vec::<EtherscanTokenTransaction>::new();

                    for i in 0..val.len() {
                        if &val[i].time_stamp.trim().parse::<u64>().unwrap_or(0) > time_stamp {
                            transactions.push(val[i].clone());
                        } else {
                            if i == 0 {
                                token_transactions
                                    .entry(chat_id)
                                    .and_modify(|map| {
                                        map.insert(w.clone(), None);
                                        ()
                                    })
                                    .or_insert_with(|| HashMap::from([(w, None)]));
                            } else {
                                token_transactions
                                    .entry(chat_id)
                                    .and_modify(|map| {
                                        map.insert(w.clone(), Some(transactions.clone()));
                                        ()
                                    })
                                    .or_insert_with(|| HashMap::from([(w, Some(transactions))]));
                            }

                            break;
                        }
                    }
                }
                Err(_) => continue,
            };
        }
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
