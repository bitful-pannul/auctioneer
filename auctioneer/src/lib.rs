use alloy_primitives::Address as EthAddress;
use alloy_signer::LocalWallet;

use frankenstein::{ChatId, SendMessageParams, TelegramApi, UpdateContent::Message as TgMessage};
use kinode_process_lib::{await_message, call_init, println, Address, Message};
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
                                        let buyer: EthAddress = EthAddress::from_str(
                                            "0x1F22784FfA5923465AC9d2D1488AB61a72bcEE65",
                                        )
                                        .unwrap();

                                        let ape_contract = EthAddress::from_str(
                                            "0xE29F8038d1A3445Ab22AD1373c65eC0a6E1161a4",
                                        )
                                        .unwrap();
                                        let ape_id = 258;
                                        let price = 500;

                                        let valid_until = std::time::SystemTime::now()
                                            .duration_since(std::time::UNIX_EPOCH)
                                            .expect("Time went backwards")
                                            .as_secs()
                                            + 60 * 60 * 24 * 7;

                                        let (uid, sig) = contracts::create_offer(
                                            wallet,
                                            &ape_contract,
                                            ape_id,
                                            &buyer,
                                            price,
                                            valid_until,
                                        )?;

                                        let sig_formatted = sig.as_bytes();
                                        let sig = hex::encode(sig_formatted);
                                        let response = format!(
                                            "Sell offer here:  with uid: {}. Here's the signature: {:?}, also valid until: {:?}",
                                             uid, sig, valid_until
                                        );
                                        params.text = response;
                                        api.send_message(&params)?;
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
