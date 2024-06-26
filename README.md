# Barter

Welcome to Barter, Kinode’s AI negotiator, an LLM assistant trained to help you sell your NFTs for the right price with none of the hassle.

Not only does Barter handle negotiation, sale, and transfer of your NFTs on telegram, but it also eliminates the listing and settlement fees of other NFT marketplaces like OpenSea.

![Barter Interface](imgs/1.jpeg)

## Requirements

- Kinode installed: [repo link](https://github.com/kinode-dao/kinode)
- Kit installed: [repo link](https://github.com/kinode-dao/kit)
- Openai API key with sufficient funds (we're using gpt-4-turbo for now, and have made efforts to keep the context as short as possible to save on costs.)
- Telegram bot API key (contact [botfather](https://telegram.me/BotFather) for keys)
- Private wallet key

## Installing from source

*Note: Only do this if you want to modify or publish the app yourself, otherwise install this from the kinode app store.*

Run `kit bs`, then visit `http://localhost:8080/main:barter:appattacc.os` to bring up the configuration when booting for the first time. In the initial configuration phase, you’ll have to add in an OpenAI API key, a Telegram Bot API key, and the wallet address which contains your NFTs.

![Barter Interface](imgs/2.jpeg)

Once the config is successfully submitted, you can add NFTs as you please by following the instructions on the NFT manager.

![Barter Interface](imgs/3.jpeg)

You can now message your TG bot at the link provided by botfather in the setup, and try to get it to make you the cheapest offer possible.

![Barter Interface](imgs/4.jpeg)

When a link has been sent by the bot, you can easily buy it f.ex. MetaMask.

![Barter Interface](imgs/5.jpeg)

## Technical Entrypoints
For developers interested in diving deeper, explore the core codebase through these files:

- [auctioneer/src/lib.rs](https://github.com/kinode-dao/barter/blob/main/auctioneer/src/lib.rs)
Main Entrypoint, where the communication with the frontend (through http), the chains, and the internal kinode messages are handled. This is where internal state is being managed, loaded and saved.

- [auctioneer/src/context.rs](https://github.com/kinode-dao/barter/blob/main/auctioneer/src/context.rs)
Main context manager for the LLM. This app can handle multiple simultaneous chats, where the core NFTs being traded are synchronized. This is also where the system prompt as well as passphrases for LLM-triggered actions are being handled.

- [Escrow Contract](https://github.com/bitful-pannul/NFTEscrow)
Escrow contract that handles approvals and seller signature verifications. Small contract, 60 lines or so!
