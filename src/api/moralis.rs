use reqwest::{header::ACCEPT, Client};
use serde::{de, Deserialize};
use std::env;

async fn send_request<T: de::DeserializeOwned>(url: String) -> Result<T, reqwest::Error> {
    let response = Client::new()
        .get(format!("https://deep-index.moralis.io/api/v2.2/{}", url))
        .header(ACCEPT, "applciation/json")
        .header(
            "X-API-Key",
            env::var("MORALIS_API").expect("MORALIS_API env var is not set"),
        )
        .send()
        .await?
        .json()
        .await?;

    Ok(response)
}

pub async fn get_token_price(contract: String) -> Result<MoralisTokenPrice, reqwest::Error> {
    send_request::<MoralisTokenPrice>(format!(
        "erc20/{}/price?chain=eth&include=percent_change",
        contract
    ))
    .await
}

pub async fn get_token_balances_with_prices(
) -> Result<MoralisResult<MoralisTokenBalancesWithPrices>, reqwest::Error> {
    send_request::<MoralisResult<MoralisTokenBalancesWithPrices>>(format!(
        "wallets/{}/tokens?chain=eth",
        env::var("ETH_ADDRESS").expect("ETH_ADDRESS env var is not set")
    ))
    .await
}

#[derive(Debug, Deserialize)]
pub struct MoralisResult<T> {
    pub cursor: Option<String>,
    pub page: u16,
    pub page_size: u16,
    pub result: Vec<T>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MoralisTokenPrice {
    pub token_name: String,
    pub token_symbol: String,
    pub token_logo: String,
    pub token_decimals: String,
    pub native_price: String,
    pub usd_price: f32,
    pub usd_price_formatted: String,
    #[serde(alias = "24hrPercentChange")]
    pub day_percent_change: String,
    pub exchange_address: String,
    pub exchange_name: String,
    pub token_address: String,
    pub to_block: String,
}

#[derive(Debug, Deserialize)]
pub struct MoralisTokenBalancesWithPrices {
    pub token_address: String,
    pub symbol: String,
    pub name: String,
    pub logo: Option<String>,
    pub thumbnail: Option<String>,
    pub decimals: u8,
    pub balance: String,
    pub possible_spam: bool,
    pub verified_contract: bool,
    pub balance_formatted: String,
    pub usd_price: f64,
    pub usd_price_24hr_percent_change: f32,
    pub usd_price_24hr_usd_change: f32,
    pub usd_value: f64,
    pub usd_value_24hr_usd_change: f32,
    pub total_supply: Option<String>,
    pub total_supply_formatted: Option<String>,
    pub percentage_relative_to_total_supply: Option<f32>,
    pub native_token: bool,
    pub portfolio_percentage: f32,
}
