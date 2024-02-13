/*
Goal:
    Create a Telegram bot that snipes crypto coins posted in another Telegram channel.
        - Listens for the call
        - Automatically swaps crypto to the posted coins
        - Does basic due diligence about the coin (honeypot, fee, market cap etc..)
        - Automatically realizes profit/loss once reaching a threshold

TODO:
    Integrate Alchemy API
        - calculation for CUs (to avoid API rate limit)
        - handle transaction
            https://excalidraw.com/ activity flow
    Telegram
        - figure out how to relay telegram messages from private group to a bot
        - filter messages so it only listens to the first call
            current idea is to start the listening process manually via the telegram bot
            automate this process (could be costly due to hosting)
        - notification of possible and executed trades
    Docker
        - learn about Docker and make a container for this app
    Find a cheap, but reilable hosting service
    Optimize code for speed
    Backtest
        - mainly time
            with and without hosting
        - optimal strategy for exiting trade
        - risk reward calculation
            tax, gas, fees, capital, liquidity/market cap
            
APIs to be used:
    - Alchemy
        - transaction execution
        - gas estimation
    - Honeypot.is
        - Token info
        (no market cap)
    - Telegram
        - listening for the signal
        - information output
*/
mod apis;
use apis::get_token_info;

fn main() {
    get_token_info("0x07DC9B6A549E4C786819F28a385FDe4D88259823");
}