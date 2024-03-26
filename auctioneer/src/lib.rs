use alloy_primitives::Address as EthAddress;
use alloy_signer::LocalWallet;
use context::NFTKey;
use frankenstein::{
    ChatId, SendMessageParams, TelegramApi, UpdateContent::ChannelPost as TgChannelPost,
    UpdateContent::Message as TgMessage,
};
use kinode_process_lib::{
    await_message, call_init, get_blob, get_state, http, println, set_state, Address, Message,
};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, str::FromStr};

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
    pub openai_key: String,
    pub telegram_bot_api_key: String,
    pub wallet_pk: String,
    pub hosted_url: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct State {
    pub config: InitialConfig,
    pub context_manager: ContextManager,
}

impl State {
    fn new(config: InitialConfig) -> Self {
        State {
            config,
            context_manager: ContextManager::new(&[]),
        }
    }

    fn fetch() -> Option<State> {
        if let Some(state_bytes) = get_state() {
            bincode::deserialize(&state_bytes).ok()
        } else {
            None
        }
    }

    fn save(&self) {
        let serialized_state = bincode::serialize(self).expect("Failed to serialize state");
        set_state(&serialized_state);
    }
}

struct Session {
    tg_api: Api,
    tg_worker: Address,
    wallet: LocalWallet,
    openai_api: OpenaiApi,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct AddNFTArgs {
    pub nft_name: String,
    pub nft_address: String,
    pub nft_id: u64,
    pub chain_id: u64,
    pub nft_description: Option<String>,
    pub sell_prompt: Option<String>,
    pub min_price: f32,
}

// TODO: Needed? 
/*
/// offerings: (nft_address, nft_id) -> (rules prompt, min_price)
type Offerings = HashMap<(Address, u64), (String, u64)>;

/// sold offerings: (nft_address, nft_id) -> (price, link)
type Sold = HashMap<(Address, u64), (u64, String)>;
 */

wit_bindgen::generate!({
    path: "wit",
    world: "process",
    exports: {
        world: Component,
    },
});

fn config(body_bytes: &[u8]) -> HttpRequestOutcome {
    let Ok(initial_config) = serde_json::from_slice::<InitialConfig>(body_bytes) else {
        return HttpRequestOutcome::None;
    };
    let _ = http::send_response(
        http::StatusCode::OK,
        Some(HashMap::from([(
            "Content-Type".to_string(),
            "application/json".to_string(),
        )])),
        b"{\"message\": \"success\"}".to_vec(),
    );
    HttpRequestOutcome::Config(initial_config)
}

fn add_nft(body_bytes: &[u8]) -> HttpRequestOutcome {
    let add_nft_args: AddNFTArgs = match serde_json::from_slice(body_bytes) {
        Ok(args) => args,
        Err(e) => {
            println!("Failed to parse AddNFTArgs: {:?}", e);
            return HttpRequestOutcome::None;
        }
    };
    let _ = http::send_response(
        http::StatusCode::OK,
        Some(HashMap::from([(
            "Content-Type".to_string(),
            "application/json".to_string(),
        )])),
        b"{\"message\": \"success\"}".to_vec(),
    );
    return HttpRequestOutcome::AddNFT(add_nft_args);
}

fn list_nfts() -> HttpRequestOutcome {
    let Some(state) = State::fetch() else {
        println!("Failed to fetch state, need to have one first before listing NFTs");
        return HttpRequestOutcome::None;
    };
    let context_manager = &state.context_manager;

    let nft_listings: Vec<_> = context_manager
        .nft_listings
        .iter()
        .map(|(key, value)| {
            serde_json::json!({
                "id": key.id,
                "chain": key.chain,
                "name": value.name,
                "min_price": value.min_price,
                "address": value.address,
                "description": value.description,
                "custom_prompt": value.custom_prompt,
            })
        })
        .collect::<Vec<_>>();

    let response_body = serde_json::to_string(&nft_listings).unwrap_or_else(|_| "[]".to_string());

    let _ = http::send_response(
        http::StatusCode::OK,
        Some(HashMap::from([(
            "Content-Type".to_string(),
            "application/json".to_string(),
        )])),
        response_body.as_bytes().to_vec(),
    );

    HttpRequestOutcome::None
}

fn remove_nft(body_bytes: &[u8]) -> HttpRequestOutcome {
    let nft_key: NFTKey = match serde_json::from_slice(body_bytes) {
        Ok(args) => args,
        Err(e) => {
            println!("Failed to parse RemoveNFTArgs: {:?}", e);
            return HttpRequestOutcome::None;
        }
    };
    let _ = http::send_response(
        http::StatusCode::OK,
        Some(HashMap::from([(
            "Content-Type".to_string(),
            "application/json".to_string(),
        )])),
        b"{\"message\": \"success\"}".to_vec(),
    );
    HttpRequestOutcome::RemoveNFT(nft_key)
}

fn handle_internal_messages(
    message: &Message,
    session: &mut Option<Session>,
) -> anyhow::Result<()> {
    let Some(session) = session else {
        println!("Session not found! Returning");
        return Ok(());
    };
    let Some(mut state) = State::fetch() else {
        println!("State not found! Returning");
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
            return _handle_internal_messages(source, body, session, &mut state);
        }
    }
}

fn _handle_internal_messages(
    source: &Address,
    body: &[u8],
    session: &mut Session,
    state: &mut State,
) -> anyhow::Result<()> {
    let Session {
        tg_api,
        tg_worker,
        openai_api,
        ..
    } = session;

    let State {
        context_manager,
        config: _, // TODO: Bitful: you can use it here :D
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
                            let mut text = context_manager.chat(msg.chat.id, &text, &openai_api)?;
                            let finalized_offer_opt = context_manager.act(msg.chat.id, &text);
                            if let Some(additional_text) =
                                &context_manager.additional_text(msg.chat.id)
                            {
                                text += additional_text;
                            }

                            if let Some(finalized_offer) = finalized_offer_opt {
                                let valid_until = std::time::SystemTime::now()
                                    .duration_since(std::time::UNIX_EPOCH)
                                    .expect("Time went backwards")
                                    .as_secs()
                                    + 3600;

                                let (uid, sig) = contracts::_create_offer(
                                    &session.wallet,
                                    &EthAddress::from_str(&finalized_offer.nft_key.address)?,
                                    finalized_offer.nft_key.id,
                                    &EthAddress::from_str(&finalized_offer.buyer_address)?,
                                    (finalized_offer.price * 1e18 as f32) as u64,
                                    valid_until,
                                )?;

                                let link = format!(
                                    "https://localhost:8080/buy?nft={}&id={}&price={}&valid={}&uid={}&sig={}&chain={}",
                                    finalized_offer.nft_key.address,
                                    finalized_offer.nft_key.id,
                                    finalized_offer.price,
                                    valid_until,
                                    uid,
                                    hex::encode(sig.as_bytes()),
                                    finalized_offer.nft_key.chain
                                );
                                println!("Purchase link: {}", link);
                                format!("buy it at the link: {}", &link)
                            } else {
                                text
                            }
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

fn fetch_status() -> HttpRequestOutcome {
    let (_state, status): (Option<State>, &str) = match State::fetch() {
        Some(state) => (Some(state), "manage-nfts"),
        None => (None, "config"),
    };

    let Ok(response) = serde_json::to_vec(&serde_json::json!({ "status": status })) else {
        println!("Failed to serialize status: {:?}", status);
        return HttpRequestOutcome::None;
    };

    let headers = HashMap::from([("Content-Type".to_string(), "application/json".to_string())]);
    let _ = http::send_response(http::StatusCode::OK, Some(headers), response);

    return HttpRequestOutcome::None;
}

fn modify_state(http_request_outcome: HttpRequestOutcome) {
    match http_request_outcome {
        HttpRequestOutcome::Config(config) => {
            State::fetch().unwrap_or_else(|| State::new(config)).save();
        }
        HttpRequestOutcome::AddNFT(add_nft_args) => {
            if let Some(mut state) = State::fetch() {
                state.context_manager.add_nft(add_nft_args);
                state.save();
            } else {
                println!("Failed to fetch state, need to have one first before adding NFTs");
            }
        }
        HttpRequestOutcome::RemoveNFT(nft_key) => {
            if let Some(mut state) = State::fetch() {
                state.context_manager.remove_nft(&nft_key);
                state.save();
            } else {
                println!("Failed to fetch state, need to have one first before removing NFTs");
            }
        }
        HttpRequestOutcome::None => {}
    }
}

fn generate_session(our: &Address, state: &State) -> anyhow::Result<Session> {
    let Ok(openai_api) = spawn_openai_pkg(our.clone(), &state.config.openai_key) else {
        return Err(anyhow::anyhow!("openAI couldn't boot."));
    };
    let Ok((tg_api, tg_worker)) =
        init_tg_bot(our.clone(), &state.config.telegram_bot_api_key, None)
    else {
        return Err(anyhow::anyhow!("tg bot couldn't boot."));
    };

    let Ok(wallet) = state.config.wallet_pk.parse::<LocalWallet>() else {
        return Err(anyhow::anyhow!("couldn't parse private key."));
    };
    Ok(Session {
        tg_api,
        tg_worker,
        wallet,
        openai_api,
    })
}

fn update_state_and_session(
    our: &Address,
    session: &mut Option<Session>,
    http_request_outcome: HttpRequestOutcome,
) -> anyhow::Result<()> {
    modify_state(http_request_outcome.clone());
    if let Some(state) = State::fetch() {
        if session.is_none() || matches!(http_request_outcome, HttpRequestOutcome::Config(_)) {
            *session = generate_session(our, &state).ok();
        }
    }

    Ok(())
}

#[derive(Clone)]
enum HttpRequestOutcome {
    Config(InitialConfig),
    AddNFT(AddNFTArgs),
    RemoveNFT(NFTKey),
    None,
}

fn handle_http_messages(message: &Message) -> HttpRequestOutcome {
    match message {
        Message::Response { .. } => {
            return HttpRequestOutcome::None;
        }
        Message::Request { ref body, .. } => {
            let Ok(server_request) = http::HttpServerRequest::from_bytes(body) else {
                return HttpRequestOutcome::None;
            };

            let Some(http_request) = server_request.request() else {
                return HttpRequestOutcome::None;
            };

            let Some(body) = get_blob() else {
                return HttpRequestOutcome::None;
            };

            let bound_path = http_request.bound_path(Some(PROCESS_ID));
            println!("on path: {:?}", bound_path);
            match bound_path {
                "/status" => {
                    return fetch_status();
                }
                "/config" => {
                    return config(&body.bytes);
                }
                "/addnft" => {
                    return add_nft(&body.bytes);
                }
                "/removenft" => {
                    return remove_nft(&body.bytes);
                }
                "/listnfts" => {
                    return list_nfts();
                }
                _ => {
                    return HttpRequestOutcome::None;
                }
            }
        }
    }
}

/// on startup, the auctioneer will need a tg token, open_ai token, and a private key holding the NFTs.
fn init(our: Address) {
    println!("initialize me!");
    let _ = http::serve_ui(
        &our,
        "ui/sell/",
        true,
        false,
        vec![
            "/",
            "/status",
            "/config",
            "/addnft",
            "/removenft",
            "/listnfts",
        ],
    );

    let _ = http::serve_ui(&our, "ui/buy/", false, false, vec!["/buy"]);

    let mut session: Option<Session> = None;
    loop {
        let Ok(message) = await_message() else {
            continue;
        };
        if message.source().node != our.node {
            continue;
        }

        if message.source().process == "http_server:distro:sys" {
            let http_request_outcome = handle_http_messages(&message);
            let _ = update_state_and_session(&our, &mut session, http_request_outcome);
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

// TODO: Zen: Break away some of the methods into another file
