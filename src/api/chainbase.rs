use reqwest::{header::CONTENT_TYPE, Client};
use serde::{de, Deserialize};
use std::env;

impl<T: de::DeserializeOwned> ChainbaseAPI<T> {
    async fn send_request(url: String) -> Result<ChainbaseAPI<T>, reqwest::Error> {
        let response = Client::new()
            .get(format!("https://api.chainbase.online/v1/{}", url))
            .header(CONTENT_TYPE, "applciation/json")
            .header(
                "x-api-key",
                env::var("CHAINBASE_API").expect("CHAINBASE_API env var is not set"),
            )
            .send()
            .await?
            .json()
            .await?;

        Ok(response)
    }

    pub async fn get_top_token_holders(
        contract: String,
    ) -> Result<ChainbaseAPI<Vec<ChainbaseTokenOwners>>, reqwest::Error> {
        ChainbaseAPI::<Vec<ChainbaseTokenOwners>>::send_request(format!(
            "token/top-holders?\
            chain_id=1\
            &contract_address={}\
            &limit=10",
            contract
        ))
        .await
    }
}

#[derive(Debug, Deserialize)]
pub struct ChainbaseAPI<T> {
    pub code: u16,
    pub message: String,
    pub data: T,
}

#[derive(Debug, Deserialize)]
pub struct ChainbaseTokenOwners {
    pub wallet_address: String,
    pub original_amount: String,
    pub amount: String,
    pub usd_value: String,
}
