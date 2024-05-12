# snipers

A telegram bot written in rust making ERC-20 token buying and selling simpler, safer and faster. Providing useful safety checks and other utilities all in one place.

## Bot functionality

- Buy and sell ERC-20 tokens quickly and easily (on-chain transactions not yet implemented)
- Check current ERC-20 tokens and ETH balances
- Monitor ETH wallets for new ERC-20 token transactions
- Check ETH gas fees and estimated uniswap transaction costs
- Scan ERC-20 tokens for potential red flags
- Snipe new tokens after they have passed basic safety checks (on-chain transactions not yet implemented, currently just sends a notification)

## Setting up your environment

To use this bot, you have to self host it.<br>
There are a few API keys you will need when setting up your environment, making sure the bot works.

> You can use every API listed here for free, if you don't go over their free tier

### Environment variables

You need to set the following environment variables.

    ETH_ADDRESS="YourETHAddress"
    TELOXIDE_TOKEN="YourTelegramBotApiKey"
    ETHERSCAN_API="YourAPIKey"
    ALCHEMY_API="YourAPIKey"
    MORALIS_API="YourAPIKey"

- For the **ETH_ADDRESS**, you should add your ethereum address.<br>
  Used for getting wallet balance, executing transactions
- For the **TELOXIDE_TOKEN**, you can follow [this tutorial](https://core.telegram.org/bots/features#creating-a-new-bot).<br>
  Used for telegram bot messaging
- For the **ETHERSCAN_API**, you can follow [this tutorial](https://docs.etherscan.io/getting-started/viewing-api-usage-statistics).<br>
  Used for getting the current price of ethereum
- For the **ALCHEMY_API**, you can follow [this tutorial](https://docs.alchemy.com/docs/alchemy-quickstart-guide#1key-create-an-alchemy-key).<br>
  Used for getting ERC-20 token balances, ETH balance, ETH gas fee
- For the **MORALIS_API**, you can follow [this tutorial](https://docs.moralis.io/web3-data-api/evm/get-your-api-key).<br>
  Used for getting ERC-20 token prices, top token holders

## Commands

> Command parameters should be seperated by one whitespace

/help &emsp;&emsp;list availabe commands<br>
/buy &emsp;&emsp; buy ERC-20 token (walletAddress: String amountInUsd: f64 slippagePercent: f32)<br>
/sell &emsp;&emsp;&nbsp; sell ERC-20 token (walletAddress: String amountInUsd: f64 slippagePercent: f32)<br>
/portfolio&nbsp; get wallet ERC-20 token balances<br>
/gas &emsp;&emsp;&nbsp; get current eth gas<br>
/watch &emsp;&nbsp; start monitoring etherum wallets (walletAddress: Vec\<String\>)<br>
/scan &emsp;&emsp; scan an ERC-20 token (contract: String)<br>
/settings &nbsp;&nbsp; change bot settings<br>
/cancel &emsp;&nbsp; cancel current command<br>
