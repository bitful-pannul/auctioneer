use alloy_consensus::{SignableTransaction, TxLegacy};
use alloy_network::TxSignerSync;
use alloy_primitives::{Address as EthAddress, Bytes, TxKind};
use alloy_signer::LocalWallet;

use frankenstein::{ChatId, SendMessageParams, TelegramApi, UpdateContent::Message as TgMessage};
use kinode_process_lib::{await_message, call_init, eth::Provider, println, Address, Message};
use std::{collections::HashMap, str::FromStr};

mod tg_api;
use prompts::create_chat_params;
use tg_api::{init_tg_bot, Api, TgResponse};

mod llm_types;
mod prompts;

mod llm_api;
use llm_api::{spawn_openai_pkg, OpenaiApi};

mod contracts;

/// context held: chat_id -> history
type ChatContexts = HashMap<i64, Vec<String>>;

/// offerings: (nft_address, nft_id) -> (rules prompt, min_price)
type Offerings = HashMap<(Address, u64), (String, u64)>;

/// sold offerings: (nft_address, nft_id) -> (price, link)
type Sold = HashMap<(Address, u64), (u64, String)>;

wit_bindgen::generate!({
    path: "wit",
    world: "process",
    exports: {
        world: Component,
    },
});

fn handle_message(
    _our: &Address,
    api: &Api,
    tg_worker: &Address,
    llm_api: &OpenaiApi,
    wallet: &mut LocalWallet,
    _chats: &mut ChatContexts,
    _offerings: &mut Offerings,
    _sold: &mut Sold,
) -> anyhow::Result<()> {
    let message = await_message()?;

    match message {
        Message::Response { .. } => {
            return Err(anyhow::anyhow!("unexpected Response: {:?}", message));
        }
        Message::Request {
            ref source,
            ref body,
            ..
        } => {
            match serde_json::from_slice(body)? {
                TgResponse::Update(tg_update) => {
                    let updates = tg_update.updates;
                    // assert update is from our worker
                    if source != tg_worker {
                        return Err(anyhow::anyhow!(
                            "unexpected source: {:?}, expected: {:?}",
                            source,
                            tg_worker
                        ));
                    }

                    if let Some(update) = updates.last() {
                        match &update.content {
                            TgMessage(msg) => {
                                // get msg contents, and branch on what to send back!
                                let text = msg.text.clone().unwrap_or_default();

                                // fill in default send_message params, switch content later!
                                let mut params = SendMessageParams {
                                    chat_id: ChatId::Integer(msg.chat.id),
                                    disable_notification: None,
                                    entities: None,
                                    link_preview_options: None,
                                    message_thread_id: None,
                                    parse_mode: None,
                                    text: "temp".to_string(),
                                    protect_content: None,
                                    reply_markup: None,
                                    reply_parameters: None,
                                    // todo, maybe change api so can ..Default::default()?
                                };

                                match text.as_str() {
                                    "/start" => {
                                        params.text = "I'm an auctioneer acting for X, I can tell u about what I have for sale currently.".to_string();
                                        api.send_message(&params)?;
                                    }
                                    "/sell" => {
                                        // hardcoded test:
                                        let buyer = EthAddress::from_str(
                                            "0x4dEdd1563Fc449e845542A2199A8028E65Bb3e34",
                                        )
                                        .unwrap();

                                        // could make for arbitrary chain_id here?
                                        let provider = Provider::new(10, 10);

                                        let seller = wallet.address();
                                        let chain_id = 10;

                                        let ape_contract = EthAddress::from_str(
                                            "0xfA14e1157F35E1dAD95dC3F822A9d18c40e360E2",
                                        )
                                        .unwrap();
                                        let ape_id = 506090;

                                        let opensea =
                                            EthAddress::from_str(contracts::SEAPORT).unwrap();

                                        let price = 2;

                                        let order = contracts::create_listing(
                                            &seller,
                                            &wallet,
                                            &ape_contract,
                                            ape_id,
                                            &buyer,
                                            price,
                                            chain_id,
                                        )?;

                                        let nonce = provider
                                            .get_transaction_count(seller, None)
                                            .map_err(|_| anyhow::anyhow!("failed to get nonce"))?;

                                        let mut tx = TxLegacy {
                                            chain_id: Some(chain_id),
                                            nonce: nonce.to::<u64>(),
                                            input: Bytes::from(order),
                                            gas_price: 100000000000, // Adjusted gas price; consider fetching current network conditions
                                            gas_limit: 100000, // Increased gas limit to accommodate contract interaction
                                            to: TxKind::Call(opensea),
                                            ..Default::default()
                                        };

                                        let mut buf = vec![];
                                        let sig = wallet.sign_transaction_sync(&mut tx)?;

                                        tx.encode_signed(&sig, &mut buf);

                                        let tx_hash = provider
                                            .send_raw_transaction(Bytes::from(buf))
                                            .map_err(|e| {
                                                anyhow::anyhow!(
                                                    "failed to send transaction: {:?}",
                                                    e
                                                )
                                            })?;

                                        println!(
                                            "got through some type of listing..., tx_hash {:?}",
                                            tx_hash
                                        );
                                    }
                                    _ => {
                                        let response = llm_api.chat(create_chat_params(&text))?;
                                        params.text = response;
                                        api.send_message(&params)?;
                                    }
                                }
                            }
                            _ => {
                                println!("got unhandled tg update: {:?}", update);
                            }
                        }
                    }
                }
                TgResponse::Error(e) => {
                    println!("error from tg worker: {:?}", e);
                }
            }
        }
    }
    Ok(())
}

call_init!(init);

/// on startup, the auctioneer will need a tg token, open_ai token, and a private key holding the NFTs.
fn init(our: Address) {
    println!("give me a tg token!");

    let msg = await_message().unwrap();
    let token_str = String::from_utf8(msg.body().to_vec()).expect("failed to parse tg token");
    println!("got tg token: {:?}", token_str);
    let (api, tg_worker) = init_tg_bot(our.clone(), &token_str, None).unwrap();

    println!("give me an openai key!");
    let msg = await_message().unwrap();
    let openai_key = String::from_utf8(msg.body().to_vec()).expect("failed to parse open_ai key");
    println!("Got openai key: {:?}", openai_key);

    println!("auctioneer: give me a private key!");
    let msg: Message = await_message().unwrap();
    let wallet_str =
        String::from_utf8(msg.body().to_vec()).expect("failed to parse private key as string");
    let mut wallet = wallet_str
        .parse::<LocalWallet>()
        .expect("failed to parse private key");

    let llm_api = spawn_openai_pkg(our.clone(), &openai_key).unwrap();

    let mut chats: ChatContexts = HashMap::new();
    let mut offerings: Offerings = HashMap::new();
    let mut sold: Sold = HashMap::new();

    loop {
        match handle_message(
            &our,
            &api,
            &tg_worker,
            &llm_api,
            &mut wallet,
            &mut chats,
            &mut offerings,
            &mut sold,
        ) {
            Ok(()) => {}
            Err(e) => {
                println!("auctioneer: error: {:?}", e);
            }
        };
    }
}
