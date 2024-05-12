use reqwest::{header::CONTENT_TYPE, Client};
use serde::{de, Deserialize, Serialize};
use serde_json;
use std::env;

impl<T: de::DeserializeOwned> AlchemyAPI<T> {
    async fn send_request(payload: AlchemyPayload) -> Result<AlchemyAPI<T>, reqwest::Error> {
        let response = Client::new()
            .post(format!(
                "https://eth-mainnet.g.alchemy.com/v2/{}",
                env::var("ALCHEMY_API").unwrap()
            ))
            .header(CONTENT_TYPE, "applciation/json")
            .body(serde_json::to_string(&payload).unwrap())
            .send()
            .await
            .expect("failed response")
            .json()
            .await?;

        Ok(response)
    }

    pub async fn get_eth_balance() -> Result<AlchemyAPI<String>, reqwest::Error> {
        let payload: AlchemyPayload = AlchemyPayload {
            params: Some(vec![
                String::from(env::var("ETH_ADDRESS").unwrap()),
                String::from("latest"),
            ]),
            method: String::from("eth_getBalance"),
            ..AlchemyPayload::default()
        };

        AlchemyAPI::send_request(payload).await
    }

    pub async fn get_eth_gas() -> Result<AlchemyAPI<String>, reqwest::Error> {
        let payload: AlchemyPayload = AlchemyPayload {
            method: String::from("eth_gasPrice"),
            ..AlchemyPayload::default()
        };

        AlchemyAPI::send_request(payload).await
    }
}

#[derive(Debug, Deserialize)]
pub struct AlchemyAPI<T> {
    pub jsonrpc: String,
    pub id: u32,
    pub result: T,
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
    method: String,
}
