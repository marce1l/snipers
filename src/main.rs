/*
Goal:
    Create a Telegram bot that snipes crypto coins posted in another Telegram channel.
        - Listens for the call
        - Automatically swaps crypto to the posted coins
        - Does basic due diligence about the coin (honeypot, fee, market cap etc..)
        - Automatically realizes profit/loss once reaching a threshold

APIs to be used:
    - Etherscan
        API key: 
    - Honeypot.is
        No Api key is needed
            - Can get tax
            - Is honepot or not
    - Telegram
*/
mod apis;
use apis::get_token_info;

fn main() {
    get_token_info();
}