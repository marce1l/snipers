#[path = "api/api.rs"]
mod api;
#[path = "crypto/crypto.rs"]
mod crypto;
#[path = "telegram/telegram.rs"]
mod telegram;
mod utils;

#[macro_use]
extern crate log;

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();

    pretty_env_logger::init();

    telegram::bot::run().await;
}
