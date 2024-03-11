use kinode_process_lib::{await_message, call_init, println, Address, Message, ProcessId, Request};

use frankenstein::{ChatId, SendMessageParams, TelegramApi, UpdateContent::Message as TgMessage};

mod tg_api;
use tg_api::{init_tg_bot, Api, TgResponse};

mod llm_types;
use llm_types::openai::{
    ChatParams, ChatRequest, LLMRequest, LLMResponse, Message as OpenaiMessage,
};

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
                                        content: text,
                                        role: "user".into(),
                                    };

                                    let chat_params = ChatParams {
                                        model: "gpt-3.5-turbo".into(),
                                        messages: vec![msg],
                                        max_tokens: Some(20),
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

fn init(our: Address) {
    println!("auctioneer: give me a tg token!");

    let message = await_message().unwrap();
    let token_str = String::from_utf8(message.body().to_vec()).unwrap_or_else(|_| "".to_string());

    let (api, worker) = init_tg_bot(our.clone(), &token_str, None).unwrap();

    println!("auctioneer: give me an openai key!");

    let message = await_message().unwrap();
    let openai_key = String::from_utf8(message.body().to_vec()).unwrap_or_else(|_| "".to_string());

    loop {
        match handle_message(&our, &api, &worker, &openai_key) {
            Ok(()) => {}
            Err(e) => {
                println!("auctioneer: error: {:?}", e);
            }
        };
    }
}
