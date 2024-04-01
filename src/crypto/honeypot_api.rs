use reqwest::Client;
use serde::{ Deserialize, Serialize };


pub fn get_token_info(contract: &str) -> TokenInfo {
    let honeypot_api: HoneypotAPI = HoneypotAPI::send_request(
        format!("https://api.honeypot.is/v2/IsHoneypot?address={}", contract)
    ).unwrap();


    TokenInfo {
        symbol: HoneypotAPI::get_token_symbol(&honeypot_api),
        name: HoneypotAPI::get_token_name(&honeypot_api),
        decimals: HoneypotAPI::get_token_deciamls(&honeypot_api),
        pair: HoneypotAPI::get_pair_type(&honeypot_api),
        pair_symbol: HoneypotAPI::get_token_pair_symbol(&honeypot_api),
        is_honeypot: HoneypotAPI::get_is_honeypot(&honeypot_api).0,
        honeypot_reason: HoneypotAPI::get_is_honeypot(&honeypot_api).1,
        buy_tax: HoneypotAPI::get_token_tax(&honeypot_api).0,
        sell_tax: HoneypotAPI::get_token_tax(&honeypot_api).1,
        liquidity: HoneypotAPI::get_pair_liquidity(&honeypot_api)
    }
}


#[derive(Debug)]
pub struct TokenInfo {
    pub symbol: String,
    pub name: String,
    pub decimals: u8,
    pub pair: String,
    pub pair_symbol: String,
    pub is_honeypot: bool,
    pub honeypot_reason: Option<String>,
    pub buy_tax: f32,
    pub sell_tax: f32,
    pub liquidity: f32
}


impl HoneypotAPI {

    #[tokio::main]
    async fn send_request(url: String) -> Result<HoneypotAPI, reqwest::Error> {

        let response: HoneypotAPI = Client::new()
        .get(url)
        .send()
        .await?
        .json()
        .await?;

        Ok(response)
    }

    fn get_token_name(api: &HoneypotAPI) -> String {
        api.token.name.to_owned()
    }

    fn get_token_symbol(api: &HoneypotAPI) -> String {
        api.token.symbol.to_owned()
    }

    fn get_token_deciamls(api: &HoneypotAPI) -> u8 {
        api.token.decimals
    }

    fn get_token_pair_symbol(api: &HoneypotAPI) -> String {
        api.with_token.symbol.to_owned()
    }

    fn get_token_tax(api: &HoneypotAPI) -> (f32, f32) {
        match api.simulation_result.as_ref() {
            Some(simulation_result) => {
                (simulation_result.buy_tax, simulation_result.sell_tax)
            },
            // In case honeypot.is simulation failed (token is honeypot), simulation result nad buy/sell tax fields are not sent
            // Buy/sell tax set to 100% as token is a honeypot, so tax should not matter
            None => {
                (100.0, 100.0)
            }
        }
    }

    fn get_is_honeypot(api: &HoneypotAPI) -> (bool, Option<String>) {
        match api.honeypot_result.as_ref() {
            Some(honeypot_result) => {
                if honeypot_result.is_honeypot {
                    (honeypot_result.is_honeypot, honeypot_result.honeypot_reason.to_owned())
                } else {
                    (honeypot_result.is_honeypot, None)
                }
            },
            None => {
                (true, Some(String::from("Warning! honeypot could not be determined as honeypot.is api did not send field")))
            },
        }
    }

    fn get_pair_liquidity(api: &HoneypotAPI) -> f32 {
        api.pair.liquidity
    }

    fn get_pair_type(api: &HoneypotAPI) -> String {
        api.pair.pair.pair_type.to_owned()
    }

}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct HoneypotAPI {
    token: Token,
    with_token: WithToken,
    simulation_success: bool,
    simulation_error: Option<String>,
    honeypot_result: Option<HoneypotResult>,
    simulation_result: Option<SimulationResult>,
    holder_analysis: Option<HolderAnalysis>,
    flags: Vec<String>,
    contract_code: Option<ContractCode>,
    chain: Chain,
    router: String,
    pair: Pair,
    pair_address: String
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct Token {
    name: String,
    symbol: String,
    decimals: u8,
    address: String,
    total_holders: u32
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct WithToken {
    name: String,
    symbol: String,
    decimals: u8,
    address: String,
    total_holders: u32
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct HoneypotResult {
    is_honeypot: bool,
    honeypot_reason: Option<String>
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct SimulationResult {
    max_buy: Option<MaxBuy>,
    max_sell: Option<MaxSell>,
    buy_tax: f32,
    sell_tax: f32,
    transfer_tax: f32,
    buy_gas: String,
    sell_gas: String
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct MaxBuy {
    token: f32,
    token_wei: String,
    with_token: f32,
    with_token_wei: String
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct MaxSell {
    token: f32,
    token_wei: String,
    with_token: f32,
    with_token_wei: String
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct HolderAnalysis {
    holders: String,
    successful: String,
    failed: String,
    siphoned: String,
    average_tax: f32,
    average_gas: f32,
    highest_tax: f32,
    high_tax_wallets: String,
    tax_distribution: Vec<TaxDistribution>,
    snipers_failed: u16,
    snipers_success: u16
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct TaxDistribution {
    tax: u16,
    count: u16
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct ContractCode {
    open_source: bool,
    root_open_source: bool,
    is_proxy: bool,
    has_proxy_calls: bool
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct Chain {
    id: String,
    name: String,
    short_name: String,
    currency: String
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct Pair {
    pair: Pair2,
    chain_id: String,
    reserves_0: String,
    reserves_1: String,
    liquidity: f32,
    router: String,
    created_at_timestamp: String,
    creation_tx_hash: String
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct  Pair2 {
    name: String,
    address: String,
    token_0: String,
    token_1: String,
    #[serde(alias = "type")]
    pair_type: String,
}