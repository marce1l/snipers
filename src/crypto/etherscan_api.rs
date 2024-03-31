use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::env;


pub async fn get_eth_price() -> f64 {
    tokio::task::spawn_blocking(|| {
        let response = EtherscanAPI::eth_price();
        let price = &response.unwrap().result.ethusd;
        price.to_owned().parse::<f64>().unwrap()
    }).await.expect("EtherscanAPI 'eth_price' method panicked")
}


impl EtherscanAPI {

    #[tokio::main]
    async fn send_request(url: String) -> Result<EtherscanAPI, reqwest::Error> {
        let response: EtherscanAPI = Client::new()
            .get(url)
            .send()
            .await?
            .json()
            .await?;

        Ok(response)
    }

    fn eth_price() -> Result<EtherscanAPI, reqwest::Error> {
        let payload: String = format!(
            "https://api.etherscan.io/api?\
            module=stats\
            &action=ethprice\
            &apikey={}", env::var("ETHERSCAN_API").unwrap()
        );

        EtherscanAPI::send_request(payload)
    }
}


#[derive(Debug, Deserialize, Serialize)]
struct EtherscanAPI {
    status: String,
    message: String,
    result: EthPrices,
}

#[derive(Debug, Deserialize, Serialize)]
struct EthPrices {
    ethbtc: String,
    ethbtc_timestamp: String,
    ethusd: String,
    ethusd_timestamp: String
}