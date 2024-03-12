use alloy_signer::LocalWallet;
use frankenstein::{ChatId, SendMessageParams, TelegramApi, UpdateContent::Message as TgMessage};
use kinode_process_lib::{await_message, call_init, println, Address, Message, ProcessId, Request};
use std::{collections::HashMap, str::FromStr};

mod tg_api;
use tg_api::{init_tg_bot, Api, TgResponse};

mod llm_types;
use llm_types::openai::{
    ChatParams, ChatRequest, LLMRequest, LLMResponse, Message as OpenaiMessage,
};

mod prompts;
use prompts::create_prompt;

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
    worker: &Address,
    openai_key: &str,
    _wallet: &LocalWallet,
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
        } => match serde_json::from_slice(body)? {
            TgResponse::Update(tg_update) => {
                let updates = tg_update.updates;
                // assert update is from our worker
                if source != worker {
                    return Err(anyhow::anyhow!(
                        "unexpected source: {:?}, expected: {:?}",
                        source,
                        worker
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
                                _ => {
                                    let msg = OpenaiMessage {
                                        content: create_prompt(&text),
                                        role: "user".into(),
                                    };

                                    let chat_params = ChatParams {
                                        model: "gpt-3.5-turbo".into(),
                                        messages: vec![msg],
                                        max_tokens: Some(200),
                                        temperature: Some(1.25),
                                        ..Default::default()
                                    };
                                    let chat_request = ChatRequest {
                                        params: chat_params,
                                        api_key: openai_key.to_string(),
                                    };
                                    let request = LLMRequest::Chat(chat_request);
                                    let msg = Request::new()
                                        .target(Address::new(
                                            "our",
                                            ProcessId::new(Some("openai"), "llm", "kinode"),
                                        ))
                                        .body(request.to_bytes())
                                        .send_and_await_response(10)??;

                                    let response = LLMResponse::parse(msg.body())?;
                                    if let LLMResponse::Chat(chat) = response {
                                        let completion = chat.to_chat_response();
                                        params.text = completion;
                                        api.send_message(&params)?;
                                    } else {
                                        return Err(anyhow::Error::msg(
                                            "Error querying OpenAI: wrong result",
                                        ));
                                    }
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
        },
    }
    Ok(())
}

call_init!(init);

/// on startup, the auctioneer will need a tg token, open_ai token, and a private key holding the NFTs.
fn init(our: Address) {
    println!("auctioneer: give me a tg token!");

    let msg = await_message().unwrap();
    let token_str = String::from_utf8(msg.body().to_vec()).expect("failed to parse tg token");
    let (api, worker) = init_tg_bot(our.clone(), &token_str, None).unwrap();

    println!("auctioneer: give me an openai key!");
    let msg = await_message().unwrap();
    let openai_key = String::from_utf8(msg.body().to_vec()).expect("failed to parse open_ai key");

    println!("auctioneer: give me a private key!");
    let msg: Message = await_message().unwrap();
    let wallet_str =
        String::from_utf8(msg.body().to_vec()).expect("failed to parse private key as string");
    let wallet = LocalWallet::from_str(&wallet_str).expect("failed to parse private key");

    let mut chats: ChatContexts = HashMap::new();
    let mut offerings: Offerings = HashMap::new();
    let mut sold: Sold = HashMap::new();

    loop {
        match handle_message(
            &our,
            &api,
            &worker,
            &openai_key,
            &wallet,
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
