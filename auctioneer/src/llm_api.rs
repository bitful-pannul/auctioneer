
use kinode_process_lib::{
    http::{send_request, send_request_await_response, Method},
    our_capabilities, spawn, Address, OnExit, ProcessId, Request, println
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::{path::PathBuf, str::FromStr};

use crate::llm_types::openai::{
    ChatParams, ChatRequest, LLMRequest, LLMResponse, Message as OpenaiMessage,
};
use crate::prompts::create_prompt;

pub fn init_openai_pkg(our: Address) -> anyhow::Result<Address> {
    let openai_pkg_path = format!("{}/pkg/openai.wasm", our.package_id());
    let our_caps = our_capabilities();
    let http_client = ProcessId::from_str("http_client:distro:sys").unwrap();

    let process_id = spawn(
        None,
        &openai_pkg_path,
        OnExit::None,
        our_caps,
        vec![http_client],
        false,
    )?;

    let worker_address = Address {
        node: our.node.clone(),
        process: process_id.clone(),
    };

    Ok(worker_address)
}

// TODO: Zen: This can later be used in llm_types, as part of the spec
pub fn chat(text: &str, openai_key: &str, llm_worker: &Address) -> anyhow::Result<String> {
    let msg = OpenaiMessage {
        content: create_prompt(text),
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
        .target(llm_worker)
        .body(request.to_bytes())
        .send_and_await_response(10)??;

    let response = LLMResponse::parse(msg.body())?;
    if let LLMResponse::Chat(chat) = response {
        Ok(chat.to_chat_response())
    } else {
        return Err(anyhow::Error::msg(
            "Error querying OpenAI: wrong result",
        ));
    }
}
