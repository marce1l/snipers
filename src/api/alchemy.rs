use reqwest::{header::CONTENT_TYPE, Client};
use serde::{de, Deserialize, Serialize};
use serde_json;
use std::{env, fmt};

impl<T: de::DeserializeOwned> AlchemyAPI<T> {
    #[tokio::main]
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

    pub fn get_eth_balance() -> Result<AlchemyAPI<String>, reqwest::Error> {
        let payload: AlchemyPayload = AlchemyPayload {
            params: Some(vec![
                String::from(env::var("ETH_ADDRESS").unwrap()),
                String::from("latest"),
            ]),
            method: String::from("eth_getBalance"),
            ..AlchemyPayload::default()
        };

        AlchemyAPI::send_request(payload)
    }

    pub fn get_eth_gas() -> Result<AlchemyAPI<String>, reqwest::Error> {
        let payload: AlchemyPayload = AlchemyPayload {
            method: String::from("eth_gasPrice"),
            ..AlchemyPayload::default()
        };

        AlchemyAPI::send_request(payload)
    }

    pub fn get_token_balances() -> Result<AlchemyAPI<TokenBalancesResult>, reqwest::Error> {
        let payload: AlchemyPayload = AlchemyPayload {
            params: Some(vec![String::from(env::var("ETH_ADDRESS").unwrap())]),
            method: String::from("alchemy_getTokenBalances"),
            ..AlchemyPayload::default()
        };

        AlchemyAPI::send_request(payload)
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

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenBalancesResult {
    pub address: String,
    pub token_balances: Vec<TokenBalance>,
}

impl fmt::Display for TokenBalance {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "contract: {}\nbalance: {}",
            self.contract_address, self.token_balance
        )
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenBalance {
    pub contract_address: String,
    pub token_balance: String,
}
