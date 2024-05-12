use crate::{
    telegram::bot::{self, SETTINGS, WATCHED_WALLETS},
    utils::{to_eth, to_gwei},
};
use chrono::{DateTime, Datelike, Duration, Utc};
use std::{collections::HashMap, sync::Arc};
use teloxide::{requests::Requester, types::ChatId, Bot};
use tokio::{sync::Mutex, time::sleep};

mod alchemy;
mod etherscan;
mod honeypot;
mod moralis;

use alchemy::AlchemyAPI;
pub use etherscan::EtherscanTokenTransaction;
use etherscan::{
    EtherscanAPI, EtherscanContractCreatorAndTxHash, EtherscanEthPrices,
    EtherscanInternalTransaction, EtherscanNormalTransaction,
};
pub use honeypot::HoneypotTokenInfo;
use moralis::MoralisTokenBalancesWithPrices;
pub use moralis::MoralisTokenOwners;

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

pub async fn get_internal_transactions(
    address: String,
    number_of_transactions: u8,
) -> Result<Vec<EtherscanInternalTransaction>, reqwest::Error> {
    match EtherscanAPI::<Vec<EtherscanInternalTransaction>>::get_internal_transactions(
        address,
        number_of_transactions,
    )
    .await
    {
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

pub async fn get_top_token_holders(
    contract: String,
) -> Result<Vec<MoralisTokenOwners>, reqwest::Error> {
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

pub async fn get_token_info(contract: String) -> Result<HoneypotTokenInfo, reqwest::Error> {
    match honeypot::get_token_info(contract).await {
        Ok(token_info) => Ok(token_info),
        Err(e) => Err(e.without_url()),
    }
}

// TODO: here? if addresses is longer than 5 (max allowed) split it into multiple requests
pub async fn get_contract_creator_and_tx_hash(
    addresses: Vec<String>,
) -> Result<Vec<EtherscanContractCreatorAndTxHash>, reqwest::Error> {
    match EtherscanAPI::<Vec<EtherscanContractCreatorAndTxHash>>::get_contract_creator_and_tx_hash(
        addresses,
    )
    .await
    {
        Ok(creators_and_hashes) => Ok(creators_and_hashes.result),
        Err(e) => Err(e.without_url()),
    }
}

pub async fn get_token_balances_with_prices() -> Result<Vec<OwnedToken>, reqwest::Error> {
    match moralis::get_token_balances_with_prices().await {
        Ok(token_balances) => Ok(to_owned_tokens(token_balances.result).await),
        Err(e) => Err(e.without_url()),
    }
}

async fn to_owned_tokens(token_balances: Vec<MoralisTokenBalancesWithPrices>) -> Vec<OwnedToken> {
    let mut tokens = vec![];

    for token in token_balances {
        let balance = token.balance.parse::<f64>().unwrap_or(0.0);

        if balance == 0.0 {
            continue;
        }

        tokens.push(OwnedToken {
            name: token.name,
            symbol: token.symbol,
            thumbnail: token.thumbnail,
            contract: token.token_address,
            balance: balance / 10.0f64.powf(token.decimals as f64),
            value_usd: token.usd_value,
            usd_price_24hr_percent_change: token.usd_price_24hr_percent_change,
            portfolio_percentage: token.portfolio_percentage,
        });
    }

    tokens
}

#[derive(Debug)]
pub struct OwnedToken {
    pub name: String,
    pub contract: String,
    pub thumbnail: Option<String>,
    pub symbol: String,
    pub balance: f64,
    pub value_usd: f64,
    pub usd_price_24hr_percent_change: f32,
    pub portfolio_percentage: f32,
}

pub async fn watch_wallets(bot: Bot) {
    let mut last_transcations = HashMap::<ChatId, HashMap<String, u64>>::new();

    loop {
        sleep(Duration::try_minutes(1).unwrap().to_std().unwrap()).await;

        let watched_wallets_guard = WATCHED_WALLETS.lock().await;
        let watched_wallets = watched_wallets_guard.clone();
        drop(watched_wallets_guard);

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
                                    })
                                    .or_insert_with(|| HashMap::from([(w, None)]));
                            } else {
                                token_transactions
                                    .entry(chat_id)
                                    .and_modify(|map| {
                                        map.insert(w.clone(), Some(transactions.clone()));
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

pub async fn new_token_alerts(bot: Bot) {
    let mut monitored_tokens: Vec<NewToken> = vec![];

    loop {
        sleep(Duration::try_minutes(1).unwrap().to_std().unwrap()).await;

        let settings_guard = SETTINGS.lock().await;
        let settings = settings_guard.clone();
        drop(settings_guard);

        if settings.is_empty() {
            continue;
        }

        for chat_id in settings.keys() {
            if !settings.get(chat_id).unwrap().snipe_new_tokens {
                continue;
            }

            // Uniswap V2 token contract address
            check_for_new_tokens(
                &mut monitored_tokens,
                String::from("0x5C69bEe701ef814a2B6a3EDD4B1652CB9cc5aA6f"),
            )
            .await;

            filter_new_tokens(&mut monitored_tokens).await;

            for token in monitored_tokens.clone() {
                if token.to_buy {
                    let _ = bot
                        .send_message(
                            *chat_id,
                            format!(
                                "💎💎💎 New token 💎💎💎\n\n\
                                This new token passed all the checks:\n❌ honeypot\n✅ liquidity locked\n✅ contract renounced\n\n\
                                Disclamer:\nThese checks can't detect everything (e.g.: delayed honeypot) Be careful and make sure to check it manually before buying!\n\n\
                                📄 Uniswap pair address: {}",
                                token.uniswap_pair_address,
                            ),
                        )
                        .await;
                }
            }
        }
    }
}

async fn is_token_honeypot(contract: String) -> Option<bool> {
    match get_token_info(contract).await {
        Ok(info) => {
            if info.is_honeypot || (info.buy_tax > 5.0 || info.sell_tax > 5.0) {
                return Some(true);
            } else {
                return Some(false);
            }
        }
        Err(_) => None,
    }
}

pub async fn is_liquidity_locked(contract: String) -> Option<bool> {
    match get_top_token_holders(contract).await {
        Ok(holders) => {
            for holder in holders {
                // TrustSwap: Team Finance Lock
                if holder.owner_address == "0xE2fE530C047f2d85298b07D9333C05737f1435fB"
                // UNCX Network Security : Token Vesting
                || holder.owner_address
                == "0xDba68f07d1b7Ca219f78ae8582C213d975c25cAf"
                {
                    return Some(true);
                }
            }

            return Some(false);
        }
        Err(_) => None,
    }
}

pub async fn is_contract_renounced(creator_address: String) -> Option<bool> {
    match get_normal_transactions(creator_address).await {
        Ok(transactions) => {
            for transaction in transactions {
                if transaction.function_name.contains("renounceOwnership") {
                    return Some(true);
                }
            }

            return Some(false);
        }
        Err(_) => None,
    }
}

async fn filter_new_tokens(monitored_tokens: &mut Vec<NewToken>) {
    #[derive(Default, Debug)]
    struct TokenCheck {
        is_honeypot: bool,
        liquidity_locked: bool,
        contract_renounced: bool,
    }
    let mut token_check: HashMap<String, TokenCheck> = HashMap::new();

    for token in monitored_tokens.clone() {
        match is_token_honeypot(token.uniswap_pair_address.clone()).await {
            Some(value) => {
                token_check.insert(
                    token.uniswap_pair_address.clone(),
                    TokenCheck {
                        is_honeypot: value,
                        ..Default::default()
                    },
                );
            }
            None => {
                token_check.insert(
                    token.uniswap_pair_address.clone(),
                    TokenCheck {
                        is_honeypot: false,
                        ..Default::default()
                    },
                );
            }
        }

        // cannot detect burned liquidity
        match is_liquidity_locked(token.contract_address).await {
            Some(value) => {
                token_check
                    .get_mut(&token.uniswap_pair_address)
                    .unwrap()
                    .liquidity_locked = value;
            }
            None => {}
        }

        match is_contract_renounced(token.creator).await {
            Some(value) => {
                token_check
                    .get_mut(&token.uniswap_pair_address)
                    .unwrap()
                    .contract_renounced = value;
            }
            None => {}
        }
    }

    monitored_tokens.retain_mut(|token| {
        if token_check
            .get(&token.uniswap_pair_address)
            .unwrap()
            .contract_renounced
        {
            if token_check
                .get(&token.uniswap_pair_address)
                .unwrap()
                .is_honeypot
            {
                return false;
            } else if token_check
                .get(&token.uniswap_pair_address)
                .unwrap()
                .liquidity_locked
            {
                token.to_buy = true;
            }
        } else {
            if Utc::now().timestamp()
                > (token.creation_timestamp + Duration::try_hours(2).unwrap().num_seconds())
            {
                return false;
            }
        }
        true
    });
}

async fn get_token_contract_from_pair_address(pair_address: String) -> Option<String> {
    match get_token_info(pair_address).await {
        Ok(info) => {
            return Some(info.contract_address);
        }
        Err(_) => {
            return None;
        }
    }
}

async fn check_for_new_tokens(monitored_tokens: &mut Vec<NewToken>, contract_address: String) {
    // limit to 5 until I solve batched get_contract_creator_and_tx_hash request sending
    match get_internal_transactions(contract_address, 5).await {
        Ok(etherscan_transactions) => {
            let mut filtered_transactions: Vec<EtherscanInternalTransaction> = vec![];

            if monitored_tokens.is_empty() {
                filtered_transactions.push(etherscan_transactions[0].clone());
            } else {
                for transaction in etherscan_transactions {
                    if monitored_tokens[monitored_tokens.len() - 1].creation_timestamp
                        >= transaction.time_stamp.parse::<i64>().unwrap()
                    {
                        break;
                    }

                    filtered_transactions.push(transaction);
                }
            }

            let contracts = filtered_transactions
                .iter()
                .map(|transaction| transaction.contract_address.clone())
                .collect();

            let mut creators: Vec<EtherscanContractCreatorAndTxHash> = vec![];
            match get_contract_creator_and_tx_hash(contracts).await {
                Ok(creator_and_hash) => {
                    creators.extend(creator_and_hash);
                }
                Err(_) => {}
            }

            for i in 0..filtered_transactions.len() {
                let uniswap_pair_address = filtered_transactions[i].to_owned().contract_address;
                let contract_address =
                    get_token_contract_from_pair_address(uniswap_pair_address.clone()).await;

                let creator = creators
                    .iter()
                    .map(|c| {
                        if &c.contract_address == &uniswap_pair_address {
                            c.contract_creator.to_owned()
                        } else {
                            String::from("")
                        }
                    })
                    .collect();

                monitored_tokens.push(NewToken {
                    uniswap_pair_address: uniswap_pair_address.to_owned(),
                    contract_address: contract_address.unwrap_or_default(),
                    creator: creator,
                    creation_timestamp: filtered_transactions[i].time_stamp.parse::<i64>().unwrap(),
                    to_buy: false,
                })
            }
        }
        Err(_) => {}
    }
}

#[derive(Debug, Clone)]
struct NewToken {
    uniswap_pair_address: String,
    contract_address: String,
    creator: String,
    creation_timestamp: i64,
    to_buy: bool,
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
        let utc_date: DateTime<Utc> = Utc::now();
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
                sleep(Duration::try_days(1).unwrap().to_std().unwrap()).await;

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
