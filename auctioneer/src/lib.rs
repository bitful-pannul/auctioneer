use alloy_primitives::{utils::format_ether, Address as EthAddress};
use alloy_sol_types::SolEvent;
use frankenstein::{
    ChatId, SendMessageParams, TelegramApi, UpdateContent::ChannelPost as TgChannelPost,
    UpdateContent::Message as TgMessage,
};
use kinode_process_lib::{
    await_message, call_init, eth, get_blob, http, println, Address, Message,
};
use std::{collections::HashMap, str::FromStr};

mod tg_api;
use tg_api::TgResponse;

mod context;
mod contracts;
mod helpers;

mod structs;
use structs::*;

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
    http::send_response(
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
    http::send_response(
        http::StatusCode::OK,
        Some(HashMap::from([(
            "Content-Type".to_string(),
            "application/json".to_string(),
        )])),
        b"{\"message\": \"success\"}".to_vec(),
    );
    return HttpRequestOutcome::AddNFT(add_nft_args);
}

fn remove_nft(body_bytes: &[u8]) -> HttpRequestOutcome {
    let nft_key: NFTKey = match serde_json::from_slice(body_bytes) {
        Ok(args) => args,
        Err(e) => {
            println!("Failed to parse RemoveNFTArgs: {:?}", e);
            return HttpRequestOutcome::None;
        }
    };
    http::send_response(
        http::StatusCode::OK,
        Some(HashMap::from([(
            "Content-Type".to_string(),
            "application/json".to_string(),
        )])),
        b"{\"message\": \"success\"}".to_vec(),
    );
    HttpRequestOutcome::RemoveNFT(nft_key)
}

fn list_nfts(state: &mut Option<State>) -> HttpRequestOutcome {
    let Some(state) = state else {
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
                "min_price": format_ether(value.min_price),
                "address": value.address,
                "description": value.description,
                "custom_prompt": value.custom_prompt,
            })
        })
        .collect::<Vec<_>>();

    let response_body = serde_json::to_string(&nft_listings).unwrap_or_else(|_| "[]".to_string());

    http::send_response(
        http::StatusCode::OK,
        Some(HashMap::from([(
            "Content-Type".to_string(),
            "application/json".to_string(),
        )])),
        response_body.as_bytes().to_vec(),
    );

    HttpRequestOutcome::None
}

fn handle_internal_messages(message: &Message, state: &mut Option<State>) -> anyhow::Result<()> {
    let Some(state) = state else {
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
            return handle_internal_request(source, body, state);
        }
    }
}

fn handle_internal_request(source: &Address, body: &[u8], state: &mut State) -> anyhow::Result<()> {
    let State {
        context_manager,
        config: _,
        tg_api,
        tg_worker,
        openai_api,
        ..
    } = state;

    let Ok(TgResponse::Update(tg_update)) = serde_json::from_slice(body) else {
        return Err(anyhow::anyhow!("unexpected response: {:?}", body));
    };

    let updates = tg_update.updates;
    // assert update is from our worker
    if source != tg_worker {
        return Err(anyhow::anyhow!(
            "unexpected source: {:?}, expected: {:?}",
            source,
            tg_worker
        ));
    }

    let Some(update) = updates.last() else {
        return Ok(());
    };

    let msg = match &update.content {
        TgMessage(msg) | TgChannelPost(msg) => msg,
        _ => return Err(anyhow::anyhow!("unexpected content: {:?}", update.content)),
    };

    let text = msg.text.clone().unwrap_or_default();

    let mut params = SendMessageParams::builder()
        .chat_id(ChatId::Integer(msg.chat.id))
        .text("temp".to_string())
        .build();

    params.text = if text == "/reset" {
        context_manager.clear(msg.chat.id);
        "Reset succesful!".to_string()
    } else {
        let mut text = context_manager.chat(msg.chat.id, &text, &openai_api)?;
        let finalized_offer_opt = context_manager.act(msg.chat.id, &text);
        if let Some(additional_text) = &context_manager.additional_text(msg.chat.id) {
            text += additional_text;
        }

        if let Some(finalized_offer) = finalized_offer_opt {
            let valid_until = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("Time went backwards")
                .as_secs()
                + 3600;

            let (uid, sig) = contracts::_create_offer(
                &state.wallet,
                &EthAddress::from_str(&finalized_offer.nft_key.address)?,
                finalized_offer.nft_key.id,
                &EthAddress::from_str(&finalized_offer.buyer_address)?,
                finalized_offer.price,
                valid_until,
            )?;

            let link = format!(
                "{}/buy?nft={}&id={}&price={}&valid={}&uid={}&sig={}&chain={}",
                state.config.hosted_url,
                finalized_offer.nft_key.address,
                finalized_offer.nft_key.id,
                finalized_offer.price,
                valid_until,
                uid,
                format!("0x{}", hex::encode(sig.as_bytes())),
                finalized_offer.nft_key.chain
            );

            format!("buy it at the link: {}", &link)
        } else {
            text
        }
    };
    tg_api.send_message(&params)?;
    state.save();
    Ok(())
}

call_init!(init);

fn fetch_status(state: &mut Option<State>) -> HttpRequestOutcome {
    let status = match state {
        Some(_) => "manage-nfts",
        None => "config",
    };

    let Ok(response) = serde_json::to_vec(&serde_json::json!({ "status": status })) else {
        println!("Failed to serialize status: {:?}", status);
        return HttpRequestOutcome::None;
    };

    let headers = HashMap::from([("Content-Type".to_string(), "application/json".to_string())]);
    http::send_response(http::StatusCode::OK, Some(headers), response);
    HttpRequestOutcome::None
}

fn update_state(
    our: &Address,
    state: &mut Option<State>,
    http_request_outcome: HttpRequestOutcome,
) {
    match http_request_outcome {
        HttpRequestOutcome::Config(config) => {
            match state {
                Some(state) => state.config = config,
                None => *state = Some(State::new(our, config)),
            }
            if let Some(ref mut state) = state {
                state.save();
            }
        }
        HttpRequestOutcome::AddNFT(add_nft_args) => match state {
            Some(state) => {
                state.context_manager.add_nft(add_nft_args);
                state.save();
            }
            None => println!("Failed to fetch state, need to have one first before adding NFTs"),
        },
        HttpRequestOutcome::RemoveNFT(nft_key) => match state {
            Some(state) => {
                state.context_manager.remove_nft(&nft_key);
                state.save();
            }
            None => println!("Failed to fetch state, need to have one first before removing NFTs"),
        },
        HttpRequestOutcome::None => {}
    }
}

fn handle_http_messages(message: &Message, state: &mut Option<State>) -> HttpRequestOutcome {
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
            let Ok(path) = http_request.path() else {
                return HttpRequestOutcome::None;
            };
            match path.as_str() {
                "/status" => {
                    return fetch_status(state);
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
                    return list_nfts(state);
                }
                _ => {
                    return HttpRequestOutcome::None;
                }
            }
        }
    }
}

fn handle_eth_message(message: &Message) -> HttpRequestOutcome {
    match message {
        Message::Response { .. } => {
            return HttpRequestOutcome::None;
        }
        Message::Request { ref body, .. } => {
            let Ok(eth_result) = serde_json::from_slice::<eth::EthSubResult>(body) else {
                return HttpRequestOutcome::None;
            };

            match eth_result {
                Ok(eth::EthSub { result, id }) => {
                    if let eth::SubscriptionResult::Log(log) = result {
                        // pre_filtered by seller. nice.s
                        let Ok((nft, nft_id, buyer, price)) =
                            contracts::NFTPurchased::abi_decode_data(&log.data, true)
                        else {
                            return HttpRequestOutcome::None;
                        };

                        let chain = match id {
                            1 => 11155111,
                            2 => 10,
                            3 => 8453,
                            4 => 42161,
                            _ => 0,
                        };

                        println!(
                            "sell event with all of these: {:?}, {:?}, {:?}, {:?}",
                            nft, nft_id, buyer, price
                        );
                        return HttpRequestOutcome::RemoveNFT(NFTKey {
                            address: nft.to_string(),
                            id: nft_id.to::<u64>(),
                            chain,
                        });
                    }
                }
                _ => return HttpRequestOutcome::None,
            }
        }
    }
    HttpRequestOutcome::None
}

/// on startup, the auctioneer will need a tg token, open_ai token, and a private key holding the NFTs.
fn init(our: Address) {
    println!("initialize me!");
    http::serve_ui(
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
    )
    .expect("sell_ui serving errored!");

    http::serve_ui(&our, "ui/buy/", false, false, vec!["/buy"]).expect("buy_ui serving errored!");

    let mut state = State::fetch();

    loop {
        let Ok(message) = await_message() else {
            continue;
        };
        if message.source().node != our.node {
            continue;
        }

        if message.source().process == "http_server:distro:sys" {
            let http_request_outcome = handle_http_messages(&message, &mut state);
            update_state(&our, &mut state, http_request_outcome);
        } else if message.source().process == "eth:distro:sys" {
            let http_request_outcome = handle_eth_message(&message);
            update_state(&our, &mut state, http_request_outcome);
        } else {
            match handle_internal_messages(&message, &mut state) {
                Ok(()) => {}
                Err(e) => {
                    println!("error: {:?}", e);
                }
            };
        }
    }
}
