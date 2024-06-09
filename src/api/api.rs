use crate::{
    telegram::bot::{self, SETTINGS, WATCHED_WALLETS},
    utils::{to_eth, to_gwei},
};
use chrono::{DateTime, Datelike, Duration, Utc};
use std::{collections::HashMap, sync::Arc};
use teloxide::{requests::Requester, types::ChatId, Bot};
use tokio::{sync::Mutex, time::sleep};

mod alchemy;
mod chainbase;
mod etherscan;
mod honeypot;
mod moralis;

use alchemy::AlchemyAPI;
use chainbase::ChainbaseAPI;
pub use chainbase::ChainbaseTokenOwners;
pub use etherscan::EtherscanTokenTransaction;
use etherscan::{
    EtherscanAPI, EtherscanContractCreatorAndTxHash, EtherscanEthPrices,
    EtherscanInternalTransaction, EtherscanNormalTransaction,
};
pub use honeypot::HoneypotTokenInfo;
use moralis::MoralisTokenBalancesWithPrices;

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
) -> Result<Vec<ChainbaseTokenOwners>, reqwest::Error> {
    match ChainbaseAPI::<Vec<ChainbaseTokenOwners>>::get_top_token_holders(contract).await {
        Ok(token_owners) => Ok(token_owners.data),
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

pub async fn get_contract_creator_and_tx_hash(
    addresses: Vec<String>,
) -> Result<Vec<EtherscanContractCreatorAndTxHash>, reqwest::Error> {
    let mut results: Vec<EtherscanContractCreatorAndTxHash> = vec![];
    let mut grouped_addresses: Vec<String> = vec![];

    for i in 0..addresses.len() {
        grouped_addresses.push(addresses[i].clone());

        if i % 5 == 0 || i == addresses.len() - 1 {
            match EtherscanAPI::<Vec<EtherscanContractCreatorAndTxHash>>::get_contract_creator_and_tx_hash(
                grouped_addresses.clone(),
            )
            .await
            {
                Ok(creators_and_hashes) => results.extend(creators_and_hashes.result),
                Err(e) => {
                    return Err(e.without_url())
                },
            };

            grouped_addresses.clear();
        }
    }

    Ok(results)
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
    let mut last_transaction_timestamps = HashMap::<ChatId, HashMap<String, u64>>::new();

    loop {
        sleep(Duration::try_minutes(1).unwrap().to_std().unwrap()).await;
        info!("New watch wallets cycle...");

        let watched_wallets_guard = WATCHED_WALLETS.lock().await;
        let watched_wallets = watched_wallets_guard.clone();
        drop(watched_wallets_guard);

        if watched_wallets.is_empty() {
            continue;
        };

        if last_transaction_timestamps.is_empty() {
            get_last_token_transaction_timestamps(
                &watched_wallets,
                &mut last_transaction_timestamps,
            )
            .await;
        } else {
            for (chat_id, wallets) in watched_wallets {
                for wallet in wallets {
                    match get_new_token_transactions(
                        wallet.to_owned(),
                        last_transaction_timestamps
                            .get(&chat_id)
                            .unwrap()
                            .get(&wallet)
                            .unwrap_or(&0),
                    )
                    .await
                    {
                        Some(transactions) => {
                            update_timestamps(
                                &mut last_transaction_timestamps,
                                chat_id,
                                wallet.to_owned(),
                                transactions[0].time_stamp.parse::<u64>().unwrap_or(0),
                            );

                            for transaction in transactions.iter().rev() {
                                let _ = bot::watched_wallet_notification(
                                    &bot,
                                    chat_id,
                                    &wallet,
                                    transaction,
                                )
                                .await;
                            }
                        }
                        None => {
                            continue;
                        }
                    }
                }
            }
        }
    }
}

async fn get_last_token_transaction_timestamps(
    watched_wallets: &HashMap<ChatId, Vec<String>>,
    last_transaction_timestamps: &mut HashMap<ChatId, HashMap<String, u64>>,
) {
    for (chat_id, wallets) in watched_wallets {
        for wallet in wallets {
            match get_token_transactions(wallet.to_owned()).await {
                Ok(transactions) => {
                    last_transaction_timestamps
                        .entry(chat_id.to_owned())
                        .and_modify(|map| {
                            map.insert(
                                wallet.to_owned(),
                                transactions[0].time_stamp.parse::<u64>().unwrap_or(0),
                            );
                        })
                        .or_insert(HashMap::from([(
                            wallet.to_owned(),
                            transactions[0].time_stamp.parse::<u64>().unwrap_or(0),
                        )]));
                }
                Err(e) => {
                    error!("get_token_transactions error: {}", e);
                    continue;
                }
            };
        }
    }
}

fn update_timestamps(
    last_transaction_timestamps: &mut HashMap<ChatId, HashMap<String, u64>>,
    chat_id: ChatId,
    wallet: String,
    timestamp: u64,
) {
    last_transaction_timestamps
        .entry(chat_id)
        .and_modify(|map| {
            map.insert(wallet.clone(), timestamp);
        })
        .or_insert(HashMap::from([(wallet, timestamp)]));
}

async fn get_new_token_transactions(
    wallet: String,
    timestamp: &u64,
) -> Option<Vec<EtherscanTokenTransaction>> {
    match get_token_transactions(wallet).await {
        Ok(transactions) => {
            let mut new_transactions = Vec::<EtherscanTokenTransaction>::new();

            for i in 0..transactions.len() {
                if &transactions[i].time_stamp.parse::<u64>().unwrap_or(0) > timestamp {
                    new_transactions.push(transactions[i].clone());
                } else {
                    break;
                }
            }

            if !new_transactions.is_empty() {
                return Some(new_transactions);
            }

            None
        }
        Err(e) => {
            error!("get_token_transactions error: {}", e);
            None
        }
    }
}

pub async fn new_token_alerts(bot: Bot) {
    let mut monitored_tokens: Vec<NewToken> = vec![];
    let mut last_removed_token = String::from("");

    loop {
        sleep(Duration::try_minutes(1).unwrap().to_std().unwrap()).await;
        info!("New token alerts cycle...");

        let settings_guard = SETTINGS.lock().await;
        let settings = settings_guard.clone();
        drop(settings_guard);

        if settings.is_empty() {
            continue;
        }

        // Uniswap V2 token contract address
        check_for_new_tokens(
            &mut monitored_tokens,
            String::from("0x5C69bEe701ef814a2B6a3EDD4B1652CB9cc5aA6f"),
        )
        .await;

        // if monitored_tokens was empty and the first element is filtered out then skip checking that token
        if last_removed_token == monitored_tokens[0].uniswap_pair_address {
            continue;
        }

        filter_new_tokens(&mut monitored_tokens, &mut last_removed_token).await;

        for token in &monitored_tokens {
            for chat_id in settings.keys() {
                if !settings.get(chat_id).unwrap().snipe_new_tokens {
                    continue;
                }

                if token.to_buy {
                    trace!("Token to buy true for: {:?}", token);
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
                Some(true)
            } else {
                Some(false)
            }
        }
        Err(e) => {
            error!("get_token_info error: {}", e);
            None
        }
    }
}

pub async fn is_liquidity_locked(contract: String) -> Option<bool> {
    match get_top_token_holders(contract).await {
        Ok(holders) => {
            for holder in holders {
                // TrustSwap: Team Finance Lock
                if holder.wallet_address == "0xE2fE530C047f2d85298b07D9333C05737f1435fB"
                // UNCX Network Security : Token Vesting
                || holder.wallet_address
                == "0xDba68f07d1b7Ca219f78ae8582C213d975c25cAf"
                {
                    return Some(true);
                }
            }

            Some(false)
        }
        Err(e) => {
            error!("get_top_token_holders error: {}", e);
            None
        }
    }
}

pub async fn is_liqudity_burned(contract: String) -> Option<bool> {
    match get_top_token_holders(contract).await {
        Ok(holders) => {
            if holders[0].wallet_address == "0x000000000000000000000000000000000000dEaD"
                && holders.len() == 1
            {
                return Some(true);
            } else {
                return Some(false);
            }
        }
        Err(e) => {
            error!("get_top_token_holders error {}", e);
            None
        }
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

            Some(false)
        }
        Err(e) => {
            error!("get_normal_transactions error: {}", e);
            None
        }
    }
}

async fn filter_new_tokens(monitored_tokens: &mut Vec<NewToken>, last_removed_token: &mut String) {
    #[derive(Default, Debug)]
    struct TokenCheck {
        is_honeypot: bool,
        liquidity_locked_or_burned: bool,
        contract_renounced: bool,
    }

    let mut token_check: HashMap<String, TokenCheck> = HashMap::new();

    for token in monitored_tokens.clone() {
        token_check.insert(token.uniswap_pair_address.clone(), TokenCheck::default());

        match is_token_honeypot(token.uniswap_pair_address.clone()).await {
            Some(value) => {
                token_check
                    .get_mut(&token.uniswap_pair_address)
                    .unwrap()
                    .is_honeypot = value;
            }
            None => {}
        }

        match is_liqudity_burned(token.uniswap_pair_address.clone()).await {
            Some(vale) => {
                token_check
                    .get_mut(&token.uniswap_pair_address)
                    .unwrap()
                    .liquidity_locked_or_burned = vale;
            }
            None => {}
        }

        match is_liquidity_locked(token.contract_address).await {
            Some(value) => {
                token_check
                    .get_mut(&token.uniswap_pair_address)
                    .unwrap()
                    .liquidity_locked_or_burned = value;
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
                *last_removed_token = token.uniswap_pair_address.clone();
                return false;
            } else if token_check
                .get(&token.uniswap_pair_address)
                .unwrap()
                .liquidity_locked_or_burned
            {
                token.to_buy = true;
            }
        } else {
            if Utc::now().timestamp()
                > (token.creation_timestamp + Duration::try_hours(2).unwrap().num_seconds())
            {
                *last_removed_token = token.uniswap_pair_address.clone();
                return false;
            }
        }
        true
    });
}

async fn get_token_contract_from_pair_address(pair_address: String) -> Option<String> {
    match get_token_info(pair_address).await {
        Ok(info) => Some(info.contract_address),
        Err(e) => {
            error!("get_token_info error: {}", e);
            None
        }
    }
}

async fn check_for_new_tokens(monitored_tokens: &mut Vec<NewToken>, contract_address: String) {
    match get_internal_transactions(contract_address, 20).await {
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
                Err(e) => {
                    error!("get_contract_creator_and_tx_hash error: {}", e);
                }
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
        Err(e) => {
            error!("get_internal_transactions error: {:?}", e);
        }
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
