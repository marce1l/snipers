use teloxide::{
    dispatching::{dialogue::{self, InMemStorage}, UpdateHandler},
    prelude::*,
    utils::command::{BotCommands, parse_command}
};
use lazy_static::lazy_static;
use core::fmt;
use std::{str::FromStr, sync::Mutex};

#[path ="../crypto/crypto.rs"]
mod crypto;
use crypto::alchemy_api;

type MyDialogue = Dialogue<State, InMemStorage<State>>;
type HandlerResult = Result<(), Box<dyn std::error::Error + Send + Sync>>;

#[derive(Clone, Debug)]
enum OrderType {
    Buy,
    Sell
}

impl fmt::Display for OrderType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            OrderType::Buy => write!(f, "buy"),
            OrderType::Sell => write!(f, "sell")
        }
    }
}

impl FromStr for OrderType {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "buy" => Ok(OrderType::Buy),
            "sell" => Ok(OrderType::Sell),
            _ => Err(())
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
            OrderType::Buy => write!(f, "📄 Contract: {}\n💰Amount: {}\n🏷 Slippage: {}\n🟢 Order type: {}", self.contract.as_ref().unwrap(), self.amount.as_ref().unwrap(), self.slippage.as_ref().unwrap(), self.order_type),
            OrderType::Sell => write!(f, "📄 Contract: {}\n💰Amount: {}\n🏷 Slippage: {}\n🔴 Order type: {}", self.contract.as_ref().unwrap(), self.amount.as_ref().unwrap(), self.slippage.as_ref().unwrap(), self.order_type)
        }
    }
}

#[derive(Clone, Default)]
pub enum State {
    #[default]
    Buy,
    Confirm,
}

#[derive(BotCommands, Clone, Debug)]
#[command(description = "These commands are supported:", rename_rule = "lowercase")]
enum Command {
    #[command(description = "help command")]
    Help,
    #[command(description = "buy ERC-20 token")]
    Buy(String),
    #[command(description = "sell ERC-20 token")]
    Sell(String),
    #[command(description = "get wallet balance")]
    Balance,
    #[command(description = "cancel current command")]
    Cancel,
}

lazy_static! {
    static ref TRADE_TOKEN: Mutex<TradeToken> = Mutex::new(TradeToken { contract: None, amount: None, slippage: None, order_type: OrderType::Buy });
}


#[tokio::main]
pub async fn main() {
    pretty_env_logger::init();
    log::info!("Starting command bot...");

    let bot = Bot::from_env();
    
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
            case![State::Buy]
                .branch(case![Command::Help].endpoint(help))
                .branch(case![Command::Buy(tt)].endpoint(trade_token))
                .branch(case![Command::Sell(tt)].endpoint(trade_token)),
        )
        .branch(case![Command::Cancel].endpoint(cancel));

    let message_handler = Update::filter_message()
        .branch(command_handler)
        .branch(case![State::Confirm].endpoint(confirm))
        .branch(dptree::endpoint(invalid_state));

    dialogue::enter::<Update, InMemStorage<State>, State, _>()
        .branch(message_handler)
}



fn validate_tradetoken_args(args: &Vec<&str>, order_type: OrderType) -> Option<TradeToken> {
    let mut trade_token: TradeToken = TradeToken { contract: None, amount: None, slippage: None, order_type: order_type };

    if args.len() != 3 {
        return None;
    }

    // etherum addresses are 42 characters long (including the 0x prefix)
    if args[0].len() == 42 {
        trade_token.contract = Some(String::from(args[0]));
    } else {
        trade_token.contract = None;
    }

    trade_token.amount = match args[1].parse() {
        Ok(v) => Some(v),
        Err(_) => None
    };

    trade_token.slippage = match args[2].parse() {
        Ok(v) => Some(v),
        Err(_) => None
    };

    let mut token = TRADE_TOKEN.lock().unwrap();
    *token = trade_token.clone();

    Some(trade_token)
}

async fn trade_token(bot: Bot, dialogue: MyDialogue, msg: Message) -> HandlerResult {
    let (command, args) = parse_command(msg.text().unwrap(), bot.get_me().await.unwrap().username()).unwrap();    
    let trade_token: Option<TradeToken> = validate_tradetoken_args(&args, OrderType::from_str(command.to_lowercase().as_str()).unwrap());
    let mut incorrect_params: bool = false;

    match trade_token.clone() {
        Some(tt) => {
            match tt.contract {
                Some(ctr) => (),
                None => {
                    incorrect_params = true;
                    bot.send_message(msg.chat.id, format!("Trade cancelled: submitted contract is incorrect!")).await?;
                }
            }

            match tt.amount {
                Some(am) => (),
                None => {
                    incorrect_params = true;
                    bot.send_message(msg.chat.id, format!("Trade cancelled: submitted amount is incorrect!")).await?;
                }
            }

            match tt.slippage {
                Some(slp) => (),
                None => {
                    incorrect_params = true;
                    bot.send_message(msg.chat.id, format!("Trade cancelled: submitted slippage is incorrect!")).await?;
                }
            }
        },
        None => {
            incorrect_params = true;
            bot.send_message(msg.chat.id, format!("Trade cancelled: submitted trade parameters are incorrect!")).await?;
        }
    };

    if !incorrect_params {
        bot.send_message(msg.chat.id, format!("{}", trade_token.clone().unwrap())).await?;
        bot.send_message(msg.chat.id, "Do you want to execute the transaction?").await?;
        
        dialogue.update(State::Confirm).await?;
    } else {
        dialogue.exit().await?;
    }
    
    Ok(())
}

async fn confirm(bot: Bot, dialogue: MyDialogue, msg: Message) -> HandlerResult {
    let response = msg.text().unwrap();

    if response == "yes" || response == "y" {
        bot.send_message(msg.chat.id, "Transaction executed!").await?;
    } else {
        bot.send_message(msg.chat.id, "Transaction was not executed!").await?;
    }

    dialogue.exit().await?;
    Ok(())
}

async fn balance(bot: Bot, dialogue: MyDialogue, msg: Message) -> HandlerResult {
    bot.send_message(msg.chat.id, format!("Your wallet balance is {}", alchemy_api::get_balance().await)).await?;
    Ok(())
}

async fn cancel(bot: Bot, dialogue: MyDialogue, msg: Message) -> HandlerResult {
    bot.send_message(msg.chat.id, "Current command is cancelled").await?;
    dialogue.exit().await?;
    Ok(())
}

async fn help(bot: Bot, dialogue: MyDialogue, msg: Message) -> HandlerResult {
    bot.send_message(msg.chat.id, Command::descriptions().to_string()).await?;
    Ok(())
}

async fn invalid_state(bot: Bot, msg: Message) -> HandlerResult {
    bot.send_message(msg.chat.id, "Type /help to see availabe commands.").await?;
    Ok(())
}