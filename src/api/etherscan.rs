use reqwest::Client;
use serde::{de, Deserialize, Serialize};
use std::env;

impl<T: de::DeserializeOwned> EtherscanAPI<T> {
    async fn send_request(url: String) -> Result<EtherscanAPI<T>, reqwest::Error> {
        let response: EtherscanAPI<T> = Client::new()
            .get(format!("https://api.etherscan.io/api?{}", url))
            .send()
            .await?
            .json()
            .await?;

        Ok(response)
    }

    pub async fn eth_price() -> Result<EtherscanAPI<EtherscanEthPrices>, reqwest::Error> {
        EtherscanAPI::send_request(format!(
            "module=stats\
            &action=ethprice\
            &apikey={}",
            env::var("ETHERSCAN_API").expect("ETHERSCAN_API env var is not set")
        ))
        .await
    }

    pub async fn get_normal_transactions(
        address: String,
    ) -> Result<EtherscanAPI<Vec<EtherscanNormalTransaction>>, reqwest::Error> {
        EtherscanAPI::send_request(format!(
            "module=account\
            &action=txlist\
            &address={}\
            &startblock=0\
            &endblock=99999999\
            &page=1\
            &offset=25\
            &sort=desc\
            &apikey={}",
            address,
            env::var("ETHERSCAN_API").expect("ETHERSCAN_API env var is not set")
        ))
        .await
    }

    pub async fn get_internal_transactions(
        address: String,
        number_of_transactions: u8,
    ) -> Result<EtherscanAPI<Vec<EtherscanInternalTransaction>>, reqwest::Error> {
        EtherscanAPI::send_request(format!(
            "module=account\
            &action=txlistinternal\
            &address={}\
            &startblock=0\
            &endblock=99999999\
            &page=1\
            &offset={}\
            &sort=desc\
            &apikey={}",
            address,
            number_of_transactions,
            env::var("ETHERSCAN_API").expect("ETHERSCAN_API env var is not set")
        ))
        .await
    }

    pub async fn get_token_transactions(
        address: String,
    ) -> Result<EtherscanAPI<Vec<EtherscanTokenTransaction>>, reqwest::Error> {
        EtherscanAPI::send_request(format!(
            "module=account\
            &action=tokentx\
            &address={}\
            &page=1\
            &offset=100\
            &startblock=0\
            &endblock=99999999\
            &sort=desc\
            &apikey={}",
            address,
            env::var("ETHERSCAN_API").expect("ETHERSCAN_API env var is not set")
        ))
        .await
    }

    pub async fn get_contract_creator_and_tx_hash(
        addresses: Vec<String>,
    ) -> Result<EtherscanAPI<Vec<EtherscanContractCreatorAndTxHash>>, reqwest::Error> {
        let contracts = addresses.join(",");

        EtherscanAPI::send_request(format!(
            "module=contract\
            &action=getcontractcreation\
            &contractaddresses={}\
            &apikey={}",
            contracts,
            env::var("ETHERSCAN_API").expect("ETHERSCAN_API env var is not set")
        ))
        .await
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

#[derive(Debug, Deserialize, Serialize, Clone)]
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

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct EtherscanInternalTransaction {
    pub block_number: String,
    pub time_stamp: String,
    pub hash: String,
    pub from: String,
    pub to: String,
    pub value: String,
    pub contract_address: String,
    pub input: String,
    #[serde(alias = "type")]
    pub transaction_type: String,
    pub gas: String,
    pub gas_used: String,
    pub trace_id: String,
    pub is_error: String,
    pub err_code: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
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

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct EtherscanContractCreatorAndTxHash {
    pub contract_address: String,
    pub contract_creator: String,
    pub tx_hash: String,
}
