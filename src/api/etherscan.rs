use reqwest::Client;
use serde::{de, Deserialize, Serialize};
use std::env;

impl<T: de::DeserializeOwned> EtherscanAPI<T> {
    async fn send_request(url: String) -> Result<EtherscanAPI<T>, reqwest::Error> {
        let response: EtherscanAPI<T> = Client::new().get(url).send().await?.json().await?;

        Ok(response)
    }

    pub async fn eth_price() -> Result<EtherscanAPI<EtherscanEthPrices>, reqwest::Error> {
        let payload: String = format!(
            "https://api.etherscan.io/api?\
            module=stats\
            &action=ethprice\
            &apikey={}",
            env::var("ETHERSCAN_API").unwrap()
        );

        EtherscanAPI::send_request(payload).await
    }

    pub async fn get_normal_transactions(
        address: String,
    ) -> Result<EtherscanAPI<Vec<EtherscanNormalTransaction>>, reqwest::Error> {
        let payload: String = format!(
            "https://api.etherscan.io/api?\
            module=account\
            &action=txlist\
            &address={}\
            &startblock=0\
            &endblock=99999999\
            &page=1\
            &offset=25\
            &sort=asc\
            &apikey={}",
            address,
            env::var("ETHERSCAN_API").unwrap()
        );

        EtherscanAPI::send_request(payload).await
    }

    pub async fn get_token_transactions(
        address: String,
    ) -> Result<EtherscanAPI<Vec<EtherscanTokenTransaction>>, reqwest::Error> {
        let payload: String = format!(
            "https://api.etherscan.io/api\
            ?module=account\
            &action=tokentx\
            &address={}\
            &page=1\
            &offset=100\
            &startblock=0\
            &endblock=99999999\
            &sort=asc\
            &apikey={}",
            address,
            env::var("ETHERSCAN_API").unwrap()
        );

        EtherscanAPI::send_request(payload).await
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct EtherscanAPI<T> {
    pub status: String,
    pub message: String,
    pub result: T,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct EtherscanEthPrices {
    pub ethbtc: String,
    pub ethbtc_timestamp: String,
    pub ethusd: String,
    pub ethusd_timestamp: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EtherscanNormalTransaction {
    pub block_number: String,
    pub time_stamp: String,
    pub hash: String,
    pub nonce: String,
    pub block_hash: String,
    pub transaction_index: String,
    pub from: String,
    pub to: String,
    pub value: String,
    pub gas: String,
    pub gas_price: String,
    pub is_error: String,
    #[serde(alias = "txreceipt_status")]
    pub txreceipt_status: String,
    pub input: String,
    pub contract_address: String,
    pub cumulative_gas_used: String,
    pub gas_used: String,
    pub confirmations: String,
    pub method_id: String,
    pub function_name: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EtherscanTokenTransaction {
    pub block_number: String,
    pub time_stamp: String,
    pub hash: String,
    pub nonce: String,
    pub block_hash: String,
    pub from: String,
    pub contract_address: String,
    pub to: String,
    pub value: String,
    pub token_name: String,
    pub token_symbol: String,
    pub token_decimal: String,
    pub transaction_index: String,
    pub gas: String,
    pub gas_price: String,
    pub gas_used: String,
    pub cumulative_gas_used: String,
    pub input: String,
    pub confirmations: String,
}
