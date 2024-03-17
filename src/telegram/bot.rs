use teloxide::{
    dispatching::{dialogue::{self, InMemStorage}, UpdateHandler},
    prelude::*,
    utils::command::BotCommands
};

#[path ="../crypto/crypto.rs"]
mod crypto;
use crypto::alchemy_api;


type MyDialogue = Dialogue<State, InMemStorage<State>>;
type HandlerResult = Result<(), Box<dyn std::error::Error + Send + Sync>>;

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
    #[command(description = "link telegram group to monitor")]
    TelegramGroup,
    #[command(description = "start trading group signals")]
    Start,
    #[command(description = "stop trading group signals")]
    Stop,
    #[command(description = "get wallet balance")]
    Balance,
    #[command(description = "get last entry")]
    LastEntry,
    #[command(description = "adjust take profit")]
    TakeProfit,
    #[command(description = "adjust stop loss")]
    StopLoss,
    #[command(description = "adjust max hold time")]
    MaxHoldTime,
    #[command(description = "all-time pnl")]
    Pnl,
    #[command(description = "cancel current command")]
    Cancel,
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
                .branch(case![Command::Buy(ctr)].endpoint(buy_token)),
        )
        .branch(case![Command::Cancel].endpoint(cancel));

    let message_handler = Update::filter_message()
        .branch(command_handler)
        .branch(case![State::Confirm].endpoint(confirm))
        .branch(dptree::endpoint(invalid_state));

    dialogue::enter::<Update, InMemStorage<State>, State, _>()
        .branch(message_handler)
}



async fn balance(bot: Bot, dialogue: MyDialogue, msg: Message) -> HandlerResult {
    bot.send_message(msg.chat.id, format!("Your wallet balance is {}", alchemy_api::get_balance().await)).await?;
    Ok(())
} 

// async fn watch_wallets()
use teloxide::utils::command::parse_command;

#[derive(Debug)]
struct BuyToken {
    contract: String,
    amount: f64,
    slippage: f32,
}

fn convert_to_buytoken(args: Vec<&str>) -> BuyToken {
    assert_eq!(args.len(), 3);
    // etherum addresses are 42 characters long (including the 0x prefix)
    assert_eq!(args[0].len(), 42);
    
    let amount: f64 = match args[1].parse() {
        Ok(v) => v,
        Err(_) => 0.0
    };

    let slippage: f32 = match args[2].parse() {
        Ok(v) => v,
        Err(_) => 0.0
    };
    
    BuyToken { contract: String::from(args[0]), amount: amount, slippage: slippage }
}

async fn buy_token(bot: Bot, dialogue: MyDialogue, msg: Message) -> HandlerResult {
    let (command, args) = parse_command(msg.text().unwrap(), bot.get_me().await.unwrap().username()).unwrap();
    let buy = convert_to_buytoken(args);

    bot.send_message(msg.chat.id, format!("contract: {}\namount: {}\nslippage: {}", buy.contract, buy.amount, buy.slippage)).await?;
    bot.send_message(msg.chat.id, "Do you want to execute the transaction?").await?;
    
    dialogue.update(State::Confirm).await?;
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
    bot.send_message(msg.chat.id, "Unable to handle the message. Type /help to see the usage.")
        .await?;
    Ok(())
}