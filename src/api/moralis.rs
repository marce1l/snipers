use reqwest::Client;
use serde::{de, Deserialize};
use std::env;

async fn send_request<T: de::DeserializeOwned>(url: String) -> Result<T, reqwest::Error> {
    let response = Client::new()
        .get(format!("https://deep-index.moralis.io/api/v2.2/{}", url))
        // .header(ACCEPT, "applciation/json")
        .header("X-API-Key", env::var("MORALIS_API").unwrap())
        .send()
        .await?
        .json()
        .await?;

    Ok(response)
}

pub async fn get_top_token_holders(contract: String) -> Result<TokenOwners, reqwest::Error> {
    send_request::<TokenOwners>(format!(
        "erc20/{}/owners?chain=eth&order=DESC&limit=10",
        contract
    ))
    .await
}

pub async fn get_token_price(contract: String) -> Result<TokenPrice, reqwest::Error> {
    send_request::<TokenPrice>(format!(
        "erc20/{}/price?chain=eth&include=percent_change",
        contract
    ))
    .await
}

#[derive(Debug, Deserialize)]
pub struct TokenOwners {
    pub cursor: Option<String>,
    pub page: u16,
    pub page_size: u16,
    pub result: Vec<TokenOwnersResult>,
}

#[derive(Debug, Deserialize)]
pub struct TokenOwnersResult {
    pub balance: String,
    pub balance_formatted: String,
    pub is_contract: bool,
    pub owner_address: String,
    pub owner_address_label: Option<String>,
    pub usd_value: String,
    pub percentage_relative_to_total_supply: f32,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenPrice {
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
