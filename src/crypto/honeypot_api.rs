use reqwest::Client;
use serde::{ Deserialize, Serialize };


impl HoneypotAPI {

    #[tokio::main]
    async fn send_request(url: &str) -> Result<HoneypotAPI, reqwest::Error> {

        let response: HoneypotAPI = Client::new()
        .get(url)
        .send()
        .await?
        .json()
        .await?;
        
        Ok(response)
    }

    fn get_token_name(api: &HoneypotAPI) -> &String {
        &api.token.name
    }

    fn get_token_symbol(api: &HoneypotAPI) -> &String {
        &api.token.symbol
    }

    fn get_token_pair_symbol(api: &HoneypotAPI) -> &String {
        &api.with_token.symbol
    }

    fn get_token_tax(api: &HoneypotAPI) -> (f32, f32) {
        (api.simulation_result.as_ref().unwrap().buy_tax, api.simulation_result.as_ref().unwrap().sell_tax)
    }

    fn get_is_honeypot(api: &HoneypotAPI) -> (bool, &Option<String>) {
        let is_honeypot: bool = api.honeypot_result.as_ref().unwrap().is_honeypot;
        
        if is_honeypot { return (is_honeypot, &api.honeypot_result.as_ref().unwrap().honeypot_reason); }
        (is_honeypot, &None)
    }

    fn get_pair_liquidity(api: &HoneypotAPI) -> &f32 {
        &api.pair.liquidity
    }

    fn get_pair_type(api: &HoneypotAPI) -> &String {
        &api.pair.pair.pair_type
    }

}


pub fn get_token_info(contract: &str) {
    let url: String = format!("https://api.honeypot.is/v2/IsHoneypot?address={contract}", contract = contract);
    let response: Result<HoneypotAPI, reqwest::Error> = HoneypotAPI::send_request(&url);
    let honeypot_api: HoneypotAPI = response.unwrap();

    println!("Symbol: {:?}", HoneypotAPI::get_token_symbol(&honeypot_api));
    println!("Name: {:?}", HoneypotAPI::get_token_name(&honeypot_api));
    println!("Pair: {:?}", HoneypotAPI::get_pair_type(&honeypot_api));
    println!("Pair symbol: {:?}", HoneypotAPI::get_token_pair_symbol(&honeypot_api));
    println!("Is honeypot: {:?}", HoneypotAPI::get_is_honeypot(&honeypot_api));
    println!("Tax: {:?}", HoneypotAPI::get_token_tax(&honeypot_api));
    println!("Liquidity: {:?}", HoneypotAPI::get_pair_liquidity(&honeypot_api));
}


/*
    Structs representing Honeypot.is API, for JSON parsing
    Note: Serialization is not used, but complier throws warning if not used
*/

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