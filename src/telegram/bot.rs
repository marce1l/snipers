use crate::api;
use chrono::{DateTime, Utc};
use core::fmt;
use lazy_static::lazy_static;
use std::{
    collections::HashMap,
    str::FromStr,
    time::{Duration, UNIX_EPOCH},
};
use teloxide::{
    dispatching::{
        dialogue::{self, GetChatId, InMemStorage},
        UpdateFilterExt, UpdateHandler,
    },
    prelude::*,
    types::{InlineKeyboardButton, InlineKeyboardMarkup, MessageId},
    utils::command::{parse_command, BotCommands},
};
use thousands::Separable;
use tokio::sync::Mutex;

type MyDialogue = Dialogue<State, InMemStorage<State>>;
type HandlerResult = Result<(), Box<dyn std::error::Error + Send + Sync>>;

#[derive(Clone, Debug)]
enum OrderType {
    Buy,
    Sell,
}

impl fmt::Display for OrderType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            OrderType::Buy => write!(f, "buy"),
            OrderType::Sell => write!(f, "sell"),
        }
    }
}

impl FromStr for OrderType {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "buy" => Ok(OrderType::Buy),
            "sell" => Ok(OrderType::Sell),
            _ => Err(()),
        }
    }
}

#[derive(Clone, Debug)]
struct TradeToken {
    contract: Option<String>,
    amount: Option<f64>,
    slippage: Option<f32>,
    order_type: OrderType,
}

impl fmt::Display for TradeToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // TradeToken will only be displayed if parameters are correct
        match self.order_type {
            OrderType::Buy => write!(
                f,
                "📄 Contract: {}\n💰Amount: {}\n🏷 Slippage: {}\n🟢 Order type: {}",
                self.contract.as_ref().unwrap(),
                self.amount.as_ref().unwrap(),
                self.slippage.as_ref().unwrap(),
                self.order_type
            ),
            OrderType::Sell => write!(
                f,
                "📄 Contract: {}\n💰Amount: {}\n🏷 Slippage: {}\n🔴 Order type: {}",
                self.contract.as_ref().unwrap(),
                self.amount.as_ref().unwrap(),
                self.slippage.as_ref().unwrap(),
                self.order_type
            ),
        }
    }
}

#[derive(Clone, Default)]
pub enum State {
    #[default]
    Start,
    Confirm,
}

#[derive(BotCommands, Clone, Debug)]
#[command(
    description = "These commands are supported:",
    rename_rule = "lowercase"
)]
enum Command {
    #[command(description = "list availabe commands")]
    Help,
    #[command(description = "buy ERC-20 token")]
    Buy(String),
    #[command(description = "sell ERC-20 token")]
    Sell(String),
    #[command(description = "get wallet ETH balance")]
    Balance,
    #[command(description = "get wallet ERC-20 token balances")]
    Tokens,
    #[command(description = "get current eth gas")]
    Gas,
    #[command(description = "start monitoring etherum wallets")]
    Watch(String),
    #[command(description = "cancel current command")]
    Cancel,
}

lazy_static! {
    static ref TRADE_TOKEN: Mutex<TradeToken> = Mutex::new(TradeToken {
        contract: None,
        amount: None,
        slippage: None,
        order_type: OrderType::Buy
    });
    pub static ref WATCHED_WALLETS: Mutex<HashMap<ChatId, Vec<String>>> =
        Mutex::new(HashMap::<ChatId, Vec<String>>::new());
}

pub async fn run() {
    pretty_env_logger::init();
    log::info!("Starting command bot...");

    let bot = Bot::from_env();
    let cloned_bot = bot.clone();

    tokio::spawn(async move { api::watch_wallets(cloned_bot).await });

    Dispatcher::builder(bot, schema())
        .dependencies(dptree::deps![InMemStorage::<State>::new()])
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;
}

fn schema() -> UpdateHandler<Box<dyn std::error::Error + Send + Sync + 'static>> {
    use dptree::case;

    let command_handler = teloxide::filter_command::<Command, _>()
        .branch(
            case![State::Start]
                .branch(case![Command::Buy(tt)].endpoint(trade_token))
                .branch(case![Command::Sell(tt)].endpoint(trade_token))
                .branch(case![Command::Balance].endpoint(get_eth_balance))
                .branch(case![Command::Tokens].endpoint(get_erc20_balances))
                .branch(case![Command::Gas].endpoint(get_eth_gas)),
        )
        .branch(case![Command::Watch(w)].endpoint(watch_wallets))
        .branch(case![Command::Help].endpoint(help))
        .branch(case![Command::Cancel].endpoint(cancel));

    let message_handler = Update::filter_message()
        .branch(command_handler)
        .branch(dptree::endpoint(invalid_state));

    let callback_query_handler =
        Update::filter_callback_query().branch(case![State::Confirm].endpoint(confirm_transaction));

    dialogue::enter::<Update, InMemStorage<State>, State, _>()
        .branch(message_handler)
        .branch(callback_query_handler)
}

fn make_yes_no_keyboard() -> InlineKeyboardMarkup {
    let buttons: Vec<Vec<InlineKeyboardButton>> = vec![vec![
        InlineKeyboardButton::callback("No", "no"),
        InlineKeyboardButton::callback("Yes", "yes"),
    ]];

    InlineKeyboardMarkup::new(buttons)
}

async fn validate_tradetoken_args(args: &Vec<&str>, order_type: OrderType) -> Option<TradeToken> {
    let mut trade_token: TradeToken = TradeToken {
        contract: None,
        amount: None,
        slippage: None,
        order_type: order_type,
    };

    if args.len() != 3 {
        return None;
    }

    // etherum addresses are 42 characters long (including the 0x prefix)
    if args[0].len() == 42 && args[0].starts_with("0x") {
        trade_token.contract = Some(String::from(args[0]));
    } else {
        trade_token.contract = None;
    }

    trade_token.amount = match args[1].parse() {
        Ok(v) => Some(v),
        Err(_) => None,
    };

    trade_token.slippage = match args[2].parse() {
        Ok(v) => Some(v),
        Err(_) => None,
    };

    let mut tt = TRADE_TOKEN.lock().await;
    *tt = trade_token.clone();

    Some(trade_token)
}

async fn validate_watchwallets_args(chat_id: ChatId, args: &Vec<&str>) -> Option<Vec<String>> {
    let mut watched_wallets: Vec<String> = vec![];

    for wallet in args {
        // etherum addresses are 42 characters long (including the 0x prefix)
        if wallet.starts_with("0x") && wallet.len() == 42 {
            watched_wallets.push(String::from(wallet.to_owned()));
        }
    }

    let mut ww = WATCHED_WALLETS.lock().await;
    *ww = HashMap::from([(chat_id, watched_wallets.clone())]);

    if watched_wallets.is_empty() {
        None
    } else {
        Some(watched_wallets)
    }
}

async fn loading_message(bot: &Bot, msg: &Message) -> MessageId {
    let loading_message = bot.send_message(msg.chat.id, "...").await;
    loading_message.unwrap().id
}

async fn trade_token(bot: Bot, dialogue: MyDialogue, msg: Message) -> HandlerResult {
    let (command, args) =
        parse_command(msg.text().unwrap(), bot.get_me().await.unwrap().username()).unwrap();
    let trade_token: Option<TradeToken> = validate_tradetoken_args(
        &args,
        OrderType::from_str(command.to_lowercase().as_str()).unwrap(),
    )
    .await;
    let mut incorrect_params: bool = false;

    match trade_token {
        Some(tt) => {
            match tt.contract {
                Some(_) => (),
                None => {
                    incorrect_params = true;
                    bot.send_message(
                        msg.chat.id,
                        format!("Trade cancelled: submitted contract is incorrect!"),
                    )
                    .await?;
                }
            }

            match tt.amount {
                Some(_) => (),
                None => {
                    incorrect_params = true;
                    bot.send_message(
                        msg.chat.id,
                        format!("Trade cancelled: submitted amount is incorrect!"),
                    )
                    .await?;
                }
            }

            match tt.slippage {
                Some(_) => (),
                None => {
                    incorrect_params = true;
                    bot.send_message(
                        msg.chat.id,
                        format!("Trade cancelled: submitted slippage is incorrect!"),
                    )
                    .await?;
                }
            }

            if !incorrect_params {
                bot.send_message(msg.chat.id, format!("{}", tt)).await?;
                bot.send_message(msg.chat.id, "Do you want to execute the transaction?")
                    .reply_markup(make_yes_no_keyboard())
                    .await?;

                dialogue.update(State::Confirm).await?;
            }
        }
        None => {
            bot.send_message(
                msg.chat.id,
                format!("Trade cancelled: submitted trade parameters are incorrect!"),
            )
            .await?;
            dialogue.exit().await?;
        }
    };

    Ok(())
}

async fn confirm_transaction(bot: Bot, dialogue: MyDialogue, q: CallbackQuery) -> HandlerResult {
    let chat_id = q.chat_id().unwrap();

    match q.clone().data {
        Some(d) => {
            bot.answer_callback_query(q.id).await?;

            bot.delete_message(chat_id, q.message.unwrap().id).await?;

            if d == "yes" {
                bot.send_message(chat_id, format!("Transaction executed!"))
                    .await?;
                // TODO: handle transaction
            } else if d == "no" {
                bot.send_message(chat_id, format!("Transaction was not executed!"))
                    .await?;
            }
        }
        None => {
            bot.send_message(
                chat_id,
                format!("Something went wrong with the button handling"),
            )
            .await?;
        }
    }

    dialogue.exit().await?;
    Ok(())
}

async fn get_eth_balance(bot: Bot, msg: Message) -> HandlerResult {
    let loading_message_id = loading_message(&bot, &msg).await;

    match api::get_eth_price().await {
        Ok(eth_price) => match api::get_eth_balance().await {
            Ok(balance) => {
                let eth_balance = balance.parse::<f64>().unwrap_or(0.0);
                let usd_balance = ((eth_balance * eth_price) * 100.0).round() / 100.0;

                bot.delete_message(msg.chat.id, loading_message_id).await?;
                bot.send_message(
                    msg.chat.id,
                    format!(
                        "Wallet balance:\n{:.4} ETH (${})",
                        eth_balance,
                        usd_balance.separate_with_commas()
                    ),
                )
                .await?;
            }
            Err(e) => {
                bot.delete_message(msg.chat.id, loading_message_id).await?;
                bot.send_message(
                    msg.chat.id,
                    format!("Something went wrong: {}\n\nPlease try again", e),
                )
                .await?;
            }
        },
        Err(e) => {
            bot.delete_message(msg.chat.id, loading_message_id).await?;
            bot.send_message(
                msg.chat.id,
                format!("Something went wrong: {}\n\nPlease try again", e),
            )
            .await?;
        }
    }

    Ok(())
}

async fn watch_wallets(bot: Bot, msg: Message) -> HandlerResult {
    let (_, args) =
        parse_command(msg.text().unwrap(), bot.get_me().await.unwrap().username()).unwrap();
    let wallets = validate_watchwallets_args(msg.chat.id, &args).await;

    match wallets {
        Some(v) => {
            // TODO: handle watching wallets

            let mut message: String = String::from("Currently watched wallets:\n");
            let mut counter: u8 = 0;

            for wallet in v {
                counter = counter + 1;
                message.push_str(&format!("\n{}. {}", counter, &wallet));
            }

            bot.send_message(msg.chat.id, message).await?;
        }
        None => {
            bot.send_message(
                msg.chat.id,
                format!("Watch wallets cancelled: submitted wallets are incorrect"),
            )
            .await?;
        }
    }

    Ok(())
}

async fn get_erc20_balances(bot: Bot, msg: Message) -> HandlerResult {
    let loading_message_id = loading_message(&bot, &msg).await;

    match api::get_token_balances().await {
        Ok(token_balances) => {
            let mut message: String = String::from("ERC-20 Token balances:\n");

            for (token, fields) in token_balances {
                message.push_str(&format!("\n{token} ({symbol})\n📄 contract: {contract}\n💰 balance: {balance} (${balance_usd})\n",
                    token = token,
                    symbol = fields.get("symbol").unwrap(),
                    contract = fields.get("contract").unwrap(),
                    balance = fields.get("balance").unwrap().separate_with_commas(),
                    balance_usd = fields.get("balance_usd").unwrap().separate_with_commas()));
            }

            bot.delete_message(msg.chat.id, loading_message_id).await?;
            bot.send_message(msg.chat.id, format!("{}", message))
                .await?;
        }
        Err(e) => {
            bot.delete_message(msg.chat.id, loading_message_id).await?;
            bot.send_message(
                msg.chat.id,
                format!("Something went wrong: {}\n\nPlease try again", e),
            )
            .await?;
        }
    }

    Ok(())
}

async fn get_eth_gas(bot: Bot, msg: Message) -> HandlerResult {
    let loading_message_id = loading_message(&bot, &msg).await;

    match api::get_eth_gas().await {
        Ok(gwei_fee) => {
            match api::get_eth_price().await {
                Ok(eth_price) => {
                    // gas estimations calculated based on cryptoneur.xyz/en/gas-fees-calculator + fees
                    let uniswap_v2: f64 = gwei_fee * 0.000000001 * eth_price * 152809.0 * 1.03;
                    let uniswap_v3: f64 = gwei_fee * 0.000000001 * eth_price * 184523.0 * 1.03;

                    let response = format!("Current eth gas is: {:.0} gwei\n\nEstimated fees:\n🦄 Uniswap V2 swap: ${:.2}\n🦄 Uniswap V3 swap: ${:.2}", gwei_fee, uniswap_v2, uniswap_v3);
                    bot.delete_message(msg.chat.id, loading_message_id).await?;
                    bot.send_message(msg.chat.id, response).await?;
                }
                Err(e) => {
                    bot.delete_message(msg.chat.id, loading_message_id).await?;
                    bot.send_message(
                        msg.chat.id,
                        format!("Something went wrong: {}\n\nPlease try again", e),
                    )
                    .await?;
                }
            }
        }
        Err(e) => {
            bot.delete_message(msg.chat.id, loading_message_id).await?;
            bot.send_message(
                msg.chat.id,
                format!("Something went wrong: {}\n\nPlease try again", e),
            )
            .await?;
        }
    }

    Ok(())
}

async fn cancel(bot: Bot, dialogue: MyDialogue, msg: Message) -> HandlerResult {
    bot.send_message(msg.chat.id, "Current command is cancelled")
        .await?;
    dialogue.exit().await?;
    Ok(())
}

async fn help(bot: Bot, msg: Message) -> HandlerResult {
    bot.send_message(msg.chat.id, Command::descriptions().to_string())
        .await?;
    Ok(())
}

async fn invalid_state(bot: Bot, msg: Message) -> HandlerResult {
    bot.send_message(msg.chat.id, "Type /help to see availabe commands.")
        .await?;
    Ok(())
}

pub async fn watched_wallet_notification(
    bot: Bot,
    chat_id: ChatId,
    wallet: String,
    transaction: &api::EtherscanTokenTransaction,
) -> HandlerResult {
    // I don't understand why, but I need to do this for the send_messgae function to accept the ChatId...
    let ch: ChatId = chat_id.into();

    let epoch_time =
        UNIX_EPOCH + Duration::from_secs(transaction.time_stamp.parse::<u64>().unwrap());
    let datetime = DateTime::<Utc>::from(epoch_time);
    let timestamp = datetime.format("%Y-%m-%d %H:%M:%S").to_string();

    bot.send_message(
        ch,
        format!(
            "🚨🚨🚨 New transaction from watched wallet 🚨🚨🚨\n\n🔎 Wallet: {}\n\n⏰ Timestamp: {}\n🔗 Transaction hash: {}\n💎 Token symbol: {}\n💎 Token name: {}\n📄 Contract: {}",
            wallet, timestamp, transaction.hash, transaction.token_symbol, transaction.token_name, transaction.contract_address
        ),
    )
    .await?;
    Ok(())
}
