use alloy_signer::LocalWallet;
use frankenstein::{
    ChatId, SendMessageParams, TelegramApi, UpdateContent::ChannelPost as TgChannelPost,
    UpdateContent::Message as TgMessage,
};
use kinode_process_lib::{await_message, call_init, println, Address, Message};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

mod tg_api;
use tg_api::{init_tg_bot, Api, TgResponse};

mod context;
mod llm_types;

mod llm_api;
use llm_api::spawn_openai_pkg;

mod contracts;

use crate::context::ContextManager;

// internal state
struct State {
    tg_api: Api,
    tg_worker: Address,
    wallet: LocalWallet,
    context_manager: ContextManager,
    offerings: Offerings,
    sold: Sold,
}

/// offerings: (nft_address, nft_id) -> (rules prompt, min_price)
type Offerings = HashMap<(Address, u64), (String, u64)>;

/// sold offerings: (nft_address, nft_id) -> (price, link)
type Sold = HashMap<(Address, u64), (u64, String)>;

/// This is temporary and will be replaced by a call to the TG API
const NFTS: [(i64, &str, f32); 3] = [
    (1, "Milady420", 0.3),
    (2, "HUMAN ONE", 0.4),
    (3, "Clock", 0.5),
];

wit_bindgen::generate!({
    path: "wit",
    world: "process",
    exports: {
        world: Component,
    },
});

fn handle_message(_our: &Address, state: &mut State) -> anyhow::Result<()> {
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
            return handle_request(source, body, state);
        }
    }
}

fn handle_request(source: &Address, body: &[u8], state: &mut State) -> anyhow::Result<()> {
    let State {
        tg_api,
        tg_worker,
        context_manager,
        ..
    } = state;

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
                    TgMessage(msg) | TgChannelPost(msg) => {
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

                        params.text = if text == "/reset" {
                            context_manager.clear(msg.chat.id);
                            "Reset succesful!".to_string()
                        } else {
                            let (text, sold) = context_manager.chat(msg.chat.id, &text)?;
                            println!("Sale has been received with {:?}", sold);
                            println!(
                                "The message list is {:?}",
                                context_manager.get_message_history(msg.chat.id).last()
                            );
                            text
                        };
                        tg_api.send_message(&params)?;
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
    Ok(())
}

#[derive(Debug, Serialize, Deserialize)]
struct Initialize {
    tg_token: String,
    openai_key: String,
    private_key: String,
}

/// Wait for Initialize command to come in either from frontend or from the CLI.
fn startup_loop(our: &Address) -> State {
    loop {
        if let Ok(msg) = await_message() {
            if msg.source() != our {
                continue;
            }

            if let Ok(init) = serde_json::from_slice::<Initialize>(msg.body()) {
                let Ok(openai_api) = spawn_openai_pkg(our.clone(), &init.openai_key) else {
                    println!("openAI couldn't boot.");
                    continue;
                };
                let Ok((tg_api, tg_worker)) = init_tg_bot(our.clone(), &init.tg_token, None) else {
                    println!("tg bot couldn't boot.");
                    continue;
                };

                let Ok(wallet) = init.private_key.parse::<LocalWallet>() else {
                    println!("couldn't parse private key.");
                    continue;
                };

                // CLI, UI:
                // - UI approve() -> send, POST (price, prompt, address, id)

                let state = State {
                    tg_api,
                    tg_worker,
                    wallet,
                    context_manager: ContextManager::new(openai_api, &NFTS),
                    offerings: HashMap::new(),
                    sold: HashMap::new(),
                };
                return state;
            }
            // add http proud
            // if msg.source().process() == "http_server:distro:sys" { }
        }
    }
}

call_init!(init);

/// on startup, the auctioneer will need a tg token, open_ai token, and a private key holding the NFTs.
fn init(our: Address) {
    println!("initialize me!");

    let mut state = startup_loop(&our);

    loop {
        match handle_message(&our, &mut state) {
            Ok(()) => {}
            Err(e) => {
                println!("auctioneer: error: {:?}", e);
            }
        };
    }
}
