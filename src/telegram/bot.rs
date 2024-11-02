use crate::{api, utils};
use chrono::{DateTime, Duration, Utc};
use core::fmt;
use lazy_static::lazy_static;
use std::{collections::HashMap, env, str::FromStr};
use teloxide::{
    dispatching::{
        dialogue::{self, GetChatId, InMemStorage},
        UpdateFilterExt, UpdateHandler,
    },
    prelude::*,
    types::{InlineKeyboardButton, InlineKeyboardMarkup, MessageId, ParseMode},
    utils::{
        command::{parse_command, BotCommands},
        html,
    },
};
use thousands::Separable;
use tokio::sync::Mutex;
use utils::hyperlinks_from_contract;

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

#[derive(Debug, Clone, Copy, Default)]
pub struct Settings {
    pub hide_zero_token_balances: bool,
    pub snipe_new_tokens: bool,
}

#[derive(Clone, Default)]
enum State {
    #[default]
    Start,
    Confirm,
    Settings,
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
    #[command(description = "get wallet ERC-20 token balances")]
    Portfolio,
    #[command(description = "get current eth gas")]
    Gas,
    #[command(description = "start monitoring etherum wallets")]
    Watch(String),
    #[command(description = "scan an ERC-20 token")]
    Scan(String),
    #[command(description = "change bot settings")]
    Settings,
    #[command(description = "cancel current command")]
    Cancel,
}

lazy_static! {
    pub static ref SETTINGS: Mutex<HashMap<ChatId, Settings>> =
        Mutex::new(HashMap::<ChatId, Settings>::new());
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
    info!("Starting telegram bot...");

    let _ = env::var("TELOXIDE_TOKEN").expect("TELOXIDE_TOKEN env var is not set");

    let bot = Bot::from_env();
    let cloned_bot = bot.clone();
    let cloned_bot2 = bot.clone();

    info!("Spawning watch_wallets...");
    tokio::spawn(async move { api::watch_wallets(cloned_bot).await });

    info!("Spawning new_token_alerts...");
    tokio::spawn(async move { api::new_token_alerts(cloned_bot2).await });

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
                .branch(case![Command::Sell(tt)].endpoint(trade_token)),
        )
        .branch(case![Command::Portfolio].endpoint(get_portfolio))
        .branch(case![Command::Gas].endpoint(get_eth_gas))
        .branch(case![Command::Scan(t)].endpoint(scan_token))
        .branch(case![Command::Settings].endpoint(change_settings))
        .branch(case![Command::Watch(w)].endpoint(watch_wallets))
        .branch(case![Command::Help].endpoint(help))
        .branch(case![Command::Cancel].endpoint(cancel));

    let message_handler = Update::filter_message()
        .branch(command_handler)
        .branch(dptree::endpoint(invalid_state));

    let callback_query_handler = Update::filter_callback_query()
        .branch(case![State::Confirm].endpoint(confirm_transaction))
        .branch(case![State::Settings].endpoint(confirm_settings));

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

fn make_settings_keyboard() -> InlineKeyboardMarkup {
    let buttons: Vec<Vec<InlineKeyboardButton>> = vec![
        vec![InlineKeyboardButton::callback(
            "Snipe new tokens",
            "snipe_new_tokens",
        )],
        vec![InlineKeyboardButton::callback(
            "Hide zero token balances",
            "hide_zero_balance",
        )],
    ];

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

    if utils::is_valid_eth_address(args[0]) {
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
        if utils::is_valid_eth_address(wallet) {
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
        Some(callback) => {
            bot.answer_callback_query(q.id).await?;

            bot.delete_message(chat_id, q.message.unwrap().id).await?;

            if callback == "yes" {
                bot.send_message(chat_id, format!("Transaction executed!"))
                    .await?;
                // TODO: handle transaction
            } else if callback == "no" {
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

async fn watch_wallets(bot: Bot, msg: Message) -> HandlerResult {
    let (_, args) =
        parse_command(msg.text().unwrap(), bot.get_me().await.unwrap().username()).unwrap();
    let wallets = validate_watchwallets_args(msg.chat.id, &args).await;

    match wallets {
        Some(value) => {
            let mut message: String = String::from("Currently watched wallets:\n");
            let mut counter: u8 = 0;

            for wallet in value {
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

async fn get_portfolio(bot: Bot, msg: Message) -> HandlerResult {
    let loading_message_id = loading_message(&bot, &msg).await;

    match api::get_token_balances_with_prices().await {
        Ok(owned_tokens) => {
            let mut message: String = String::from("Portfolio:\n");
            let mut found = false;

            for token in owned_tokens {
                if SETTINGS
                    .lock()
                    .await
                    .get(&msg.chat.id)
                    .unwrap_or(&Settings {
                        ..Default::default()
                    })
                    .hide_zero_token_balances
                    && token.value_usd == 0.0
                {
                    continue;
                }

                let percent_change = {
                    if token.usd_price_24hr_percent_change > 0.0 {
                        format!("📈 +{:.2}%", token.usd_price_24hr_percent_change)
                    } else {
                        format!("📉 {:.2}%", token.usd_price_24hr_percent_change)
                    }
                };

                // TODO: add thumbnail to message if available
                message.push_str(&format!(
                    "\n💎 {} ({})\n💰 {} (${})\n{}\n📊 {:.2}%\n{}\n",
                    token.name,
                    token.symbol,
                    format!("{:.2}", token.balance).separate_with_commas(),
                    format!("{:.2}", token.value_usd).separate_with_commas(),
                    percent_change,
                    token.portfolio_percentage,
                    hyperlinks_from_contract(&token.contract)
                ));

                found = true;
            }

            bot.delete_message(msg.chat.id, loading_message_id).await?;
            if found {
                bot.send_message(msg.chat.id, format!("{}", message))
                    .parse_mode(ParseMode::Html)
                    .disable_web_page_preview(true)
                    .await?;
            } else {
                bot.send_message(msg.chat.id, format!("No token balances were found!"))
                    .await?;
            }
        }
        Err(e) => {
            error!("get_token_balances_with_prices error: {}", e);
            bot.delete_message(msg.chat.id, loading_message_id).await?;
            bot.send_message(
                msg.chat.id,
                format!("Something went wrong, please try again later"),
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
                    error!("get_eth_price error: {}", e);
                    bot.delete_message(msg.chat.id, loading_message_id).await?;
                    bot.send_message(
                        msg.chat.id,
                        format!("Something went wrong, please try again later"),
                    )
                    .await?;
                }
            }
        }
        Err(e) => {
            error!("get_eth_gas error: {}", e);
            bot.delete_message(msg.chat.id, loading_message_id).await?;
            bot.send_message(
                msg.chat.id,
                format!("Something went wrong, please try again later"),
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
    bot: &Bot,
    chat_id: ChatId,
    wallet: &String,
    transaction: &api::EtherscanTokenTransaction,
) -> HandlerResult {
    let epoch_time = DateTime::UNIX_EPOCH
        + Duration::try_seconds(transaction.time_stamp.parse::<i64>().unwrap()).unwrap();
    let datetime = DateTime::<Utc>::from(epoch_time);
    let timestamp = datetime.format("%Y-%m-%d %H:%M:%S").to_string();

    bot.send_message(
        chat_id,
        format!(
            "🚨🚨🚨 New transaction 🚨🚨🚨\n\n🔎 {}\n\n💎 {} ({})\n⏰ (UTC) {}\n{} | {}",
            wallet,
            transaction.token_name,
            transaction.token_symbol,
            timestamp,
            html::link(
                &format!("https://etherscan.io/tx/{}", transaction.hash),
                "Tx"
            ),
            hyperlinks_from_contract(&transaction.contract_address)
        ),
    )
    .parse_mode(ParseMode::Html)
    .disable_web_page_preview(true)
    .await?;

    Ok(())
}

async fn scan_token(bot: Bot, msg: Message) -> HandlerResult {
    let loading_message_id = loading_message(&bot, &msg).await;
    let contract = parse_command(msg.text().unwrap(), bot.get_me().await.unwrap().username())
        .unwrap()
        .1
        .join("");

    if utils::is_valid_eth_address(contract.trim()) {
        match api::get_token_info(contract.trim().to_owned()).await {
            Ok(token_info) => {
                let mut warning = false;
                let mut info = format!(
                    "Scan result for: \n📄 {}\n\n💎 {} ({})\n⚖️ ({}%, {}%)\n💵 ${}\n{}\n\n🚨 Warnings:",
                    token_info.contract_address,
                    token_info.name,
                    token_info.symbol,
                    token_info.buy_tax,
                    token_info.sell_tax,
                    token_info.liquidity.floor().separate_with_commas(),
                    hyperlinks_from_contract(&token_info.contract_address)
                );

                if token_info.is_honeypot {
                    info = format!(
                        "{}\n❌ {}",
                        info,
                        token_info
                            .honeypot_reason
                            .unwrap_or(String::from("TOKEN IS A HONEYPOT"))
                    );
                    warning = true;
                }

                if token_info.flags_description.is_some() {
                    for desc in token_info.flags_description.clone().unwrap() {
                        info = format!("{}\n❌ {}", info, desc);
                    }
                    warning = true;
                }

                if token_info.has_proxy_calls.unwrap_or(false) {
                    info = info + "\n❌ Contract has proxy calls!";
                    warning = true;
                }

                if !token_info.is_open_source.unwrap_or(true) {
                    info = info + "\n❌ Contract is not open source!";
                    warning = true;
                }

                if token_info.liquidity < 5000.0 {
                    info = info + "\n❌ Liquidity is very small!";
                    warning = true;
                }

                match api::is_contract_renounced(token_info.contract_address.clone()).await {
                    Some(response) => {
                        if !response {
                            info = info + "\n❌ Contract is not renounced!";
                            warning = true;
                        }
                    }
                    None => {}
                }

                match api::is_liquidity_locked(token_info.contract_address.clone()).await {
                    Some(response) => {
                        if !response {
                            info = info + "\n❌ Liquidity might not be locked!";
                            warning = true;
                        }
                    }
                    None => {}
                }

                if !warning {
                    info = info + "\n✅ There were no warnings found";
                }

                bot.delete_message(msg.chat.id, loading_message_id).await?;
                bot.send_message(msg.chat.id, info)
                    .parse_mode(ParseMode::Html)
                    .disable_web_page_preview(true)
                    .await?;
            }
            Err(e) => {
                error!("get_token_info error: {}", e);
                bot.send_message(
                    msg.chat.id,
                    format!("Something went wrong, please try again later"),
                )
                .await?;
            }
        }
    } else {
        bot.delete_message(msg.chat.id, loading_message_id).await?;
        bot.send_message(msg.chat.id, format!("The submitted contract is not valid!"))
            .await?;
    }

    Ok(())
}

async fn change_settings(bot: Bot, msg: Message, dialogue: MyDialogue) -> HandlerResult {
    bot.send_message(msg.chat.id, "Settings:")
        .reply_markup(make_settings_keyboard())
        .await?;
    dialogue.update(State::Settings).await?;

    Ok(())
}

async fn confirm_settings(bot: Bot, dialogue: MyDialogue, q: CallbackQuery) -> HandlerResult {
    let chat_id = q.chat_id().unwrap();
    let mut settings = SETTINGS.lock().await;
    let mut change_settings: HashMap<ChatId, Settings> = settings.to_owned();

    match q.data {
        Some(callback) => {
            bot.answer_callback_query(q.id).await?;

            // TODO: figure out how to accept multiple callbackQuerys without being stuck in the settings state
            bot.delete_message(chat_id, q.message.unwrap().id).await?;

            if callback == "hide_zero_balance" {
                change_settings
                    .entry(chat_id.clone())
                    .and_modify(|value| {
                        value.hide_zero_token_balances = !value.hide_zero_token_balances
                    })
                    .or_insert(Settings {
                        hide_zero_token_balances: true,
                        ..Default::default()
                    });

                if !change_settings
                    .get(&chat_id)
                    .unwrap()
                    .hide_zero_token_balances
                {
                    bot.send_message(chat_id, format!("Zero token balances are NOT hidden!"))
                        .await?;
                } else {
                    bot.send_message(chat_id, format!("Zero token balances are hidden!"))
                        .await?;
                }
            } else if callback == "snipe_new_tokens" {
                change_settings
                    .entry(chat_id.clone())
                    .and_modify(|value| value.snipe_new_tokens = !value.snipe_new_tokens)
                    .or_insert(Settings {
                        snipe_new_tokens: true,
                        ..Default::default()
                    });

                if !change_settings.get(&chat_id).unwrap().snipe_new_tokens {
                    bot.send_message(chat_id, format!("New tokens are NOT sniped!"))
                        .await?;
                } else {
                    bot.send_message(chat_id, format!("New tokens are sniped!"))
                        .await?;
                }
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

    *settings = change_settings;
    dialogue.exit().await?;

    Ok(())
}
