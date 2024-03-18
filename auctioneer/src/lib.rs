use alloy_signer::LocalWallet;
use frankenstein::{
    ChatId, SendMessageParams, TelegramApi, UpdateContent::ChannelPost as TgChannelPost,
    UpdateContent::Message as TgMessage,
};
use kinode_process_lib::{
    await_message, call_init, get_blob, get_state, http, println, set_state, Address, Message,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

mod tg_api;
use tg_api::{init_tg_bot, Api, TgResponse};

mod context;
mod llm_types;

mod llm_api;
use llm_api::{spawn_openai_pkg, OpenaiApi};

mod contracts;

use crate::context::ContextManager;

const PROCESS_ID: &str = "auctioneer:auctioneer:template.os";

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
struct InitialConfig {
    openai_key: String,
    telegram_bot_api_key: String,
    private_wallet_address: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct State {
    pub config: InitialConfig,
    pub context_manager: ContextManager,
}

impl State {
    fn fetch() -> Option<State> {
        if let Some(state_bytes) = get_state() {
            bincode::deserialize(&state_bytes).expect("Failed to deserialize state")
        } else {
            None
        }
    }
}

struct Session {
    tg_api: Api,
    tg_worker: Address,
    _wallet: LocalWallet,
    context_manager: ContextManager,
    openai_api: OpenaiApi,
    // TODO: Zena: Offerings and sold should probably go into context_manager.
    _offerings: Offerings,
    _sold: Sold,
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


// fn initialize_or_restore_session(our: &Address, state: Option<State>) -> anyhow::Result<Session> {
//     let (initial_config, context_manager_opt) = match state {
//         Some(state) => (state.config, Some(state.context_manager)),
//         None => {
//             let initial_config = receive_config_message(our);
//             (initial_config, None)
//         }
//     };

//     let Ok(openai_api) = spawn_openai_pkg(our.clone(), &initial_config.openai_key) else {
//         return Err(anyhow::anyhow!("openAI couldn't boot."));
//     };
//     let Ok((tg_api, tg_worker)) =
//         init_tg_bot(our.clone(), &initial_config.telegram_bot_api_key, None)
//     else {
//         return Err(anyhow::anyhow!("tg bot couldn't boot."));
//     };

//     let Ok(wallet) = initial_config.private_wallet_address.parse::<LocalWallet>() else {
//         return Err(anyhow::anyhow!("couldn't parse private key."));
//     };

//     let context_manager = match context_manager_opt {
//         Some(cm) => cm,
//         None => {
//             let cm = ContextManager::new(&NFTS);
//             let state = State {
//                 config: initial_config.clone(),
//                 context_manager: cm.clone(),
//             };
//             let serialized_state = bincode::serialize(&state).expect("Failed to serialize state");
//             set_state(&serialized_state);
//             cm
//         }
//     };

//     Ok(Session {
//         tg_api,
//         tg_worker,
//         _wallet: wallet,
//         context_manager,
//         openai_api,
//         _offerings: HashMap::new(),
//         _sold: HashMap::new(),
//     })
// }

fn config(body_bytes: &[u8]) -> Option<State> {
    let initial_config = serde_json::from_slice::<InitialConfig>(body_bytes).ok()?;
    let _ = http::send_response(
        http::StatusCode::OK,
        Some(HashMap::from([(
            "Content-Type".to_string(),
            "application/json".to_string(),
        )])),
        b"{\"message\": \"success\"}".to_vec(),
    );

    match State::fetch() {
        Some(mut state) => {
            state.config = initial_config;
            return Some(state);
        },
        None => {
            let state = State {
                config: initial_config,
                context_manager: ContextManager::new(&NFTS),
            };
            let serialized_state = bincode::serialize(&state).expect("Failed to serialize state");
            set_state(&serialized_state);
            return Some(state);
        },
    }
}

fn add_nft(body_bytes: &[u8]) -> Option<State> {
    return None; // TODO: Zen: Implement
}

fn handle_internal_messages(
    message: &Message,
    session: &mut Option<Session>,
) -> anyhow::Result<()> {
    let Some(session) = session else {
        return Ok(());
    };

    match message {
        Message::Response { .. } => {
            return Err(anyhow::anyhow!("unexpected Response: {:?}", message));
        }
        Message::Request {
            ref source,
            ref body,
            ..
        } => {
            return _handle_internal_messages(source, body, session);
        }
    }
}

fn _handle_internal_messages(
    source: &Address,
    body: &[u8],
    session: &mut Session,
) -> anyhow::Result<()> {
    let Session {
        tg_api,
        tg_worker,
        context_manager,
        openai_api,
        ..
    } = session;

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
                            let (text, sold) = context_manager.chat(msg.chat.id, &text, &openai_api)?;
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

call_init!(init);


fn fetch_status(our: &Address, msg: &Message) -> Option<State> {
    if msg.source().node != our.node {
        return None;
    }
    let (state_, status): (Option<State>, &str) = match State::fetch() {
        Some(state) => (Some(state), "manage-nfts"),
        None => (None, "config"),
    };

    let Ok(response) = serde_json::to_vec(&serde_json::json!({ "status": status })) else {
        println!("Failed to serialize status: {:?}", status);
        return None;
    };

    let headers = HashMap::from([("Content-Type".to_string(), "application/json".to_string())]);
    let _ = http::send_response(http::StatusCode::OK, Some(headers), response);

    state
}

fn handle_http_messages(our: &Address, message: &Message) -> Option<State> {
    match message {
        Message::Response { .. } => {
            return None;
        }
        Message::Request {
            ref source,
            ref body,
            ref metadata,
            ..
        } => {
            let server_request = http::HttpServerRequest::from_bytes(body).ok()?;
            let http_request = server_request
                .request()?;

            let body = get_blob()?;
            let bound_path = http_request.bound_path(Some(PROCESS_ID));
            // TODO: Zen: Later on we use a superstruct with state and other fields, or an enum
            let state = match bound_path {
                "/status" => {
                    return fetch_status(our, message);
                }
                "/config" => {
                    return config(&body.bytes);
                }
                "/addnft" => {
                    return add_nft(&body.bytes);
                }
                _ => {
                    return None;
                }
            };
        }
    }
}

/// on startup, the auctioneer will need a tg token, open_ai token, and a private key holding the NFTs.
fn init(our: Address) {
    println!("initialize me!");
    let _ = http::serve_index_html(
        &our,
        "ui",
        true,
        false,
        vec!["/", "/status", "/config", "/addnft"],
    );
    let mut session: Option<Session> = None;
    loop {
        let Ok(message) = await_message() else {
            continue;
        };
        if message.source().node != our.node {
            continue;
        }

        if message.source().process == "http_server:distro:sys" {
            let state = handle_http_messages(&our, &message);
            modify_session(&mut session, state);
            // TODO: Zen: Update session from state
        } else {
            match handle_internal_messages(&message, &mut session) {
                Ok(()) => {}
                Err(e) => {
                    println!("auctioneer: error: {:?}", e);
                }
            };
        }
    }
}
