#[path = "api/api.rs"]
mod api;
#[path = "crypto/crypto.rs"]
mod crypto;
#[path = "telegram/telegram.rs"]
mod telegram;
mod utils;

#[macro_use]
extern crate log;
use std::env;

#[tokio::main]
async fn main() {
    // for teloxide logging
    // env::set_var("RUST_LOG", "trace");
    // for snipers logging
    env::set_var("RUST_LOG", "snipers=info");

    pretty_env_logger::init();

    env::set_var("PORT", "");
    env::set_var("WEBHOOK_URL", "");

    env::set_var("ALCHEMY_API", "");
    env::set_var("TELOXIDE_TOKEN", "");
    env::set_var("ETH_ADDRESS", "");
    env::set_var("ETHERSCAN_API", "");
    env::set_var("MORALIS_API", "");
    env::set_var("CHAINBASE_API", "");

    telegram::bot::run().await;
}
