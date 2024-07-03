/// API for the bot and the parent process.
use frankenstein::{GetUpdatesParams, TelegramApi};
use kinode_process_lib::{
    http::{send_request, send_request_await_response, Method},
    our_capabilities, println, spawn, Address, OnExit, ProcessId, Request,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::str::FromStr;

use llm_interface::openai::{LLMRequest, RegisterApiKeyRequest};
use telegram_interface::*;


#[allow(unused)]
pub fn init_openai(our: Address, api_key: &str) -> anyhow::Result<Address> {
    let openai_wasm_path = format!("{}/pkg/openai.wasm", our.package_id());

    let our_caps = our_capabilities();
    let http_client = ProcessId::from_str("http_client:distro:sys").unwrap();

    let process_id = spawn(
        None,
        &openai_wasm_path,
        OnExit::Restart,
        our_caps,
        vec![http_client],
        false,
    )?;

    let worker_address = Address {
        node: our.node.clone(),
        process: process_id.clone(),
    };

    let api_message = LLMRequest::RegisterOpenaiApiKey(RegisterApiKeyRequest {
        api_key: api_key.to_string(),
    });
    let _ = Request::to(worker_address.clone())
        .body(serde_json::to_vec(&api_message)?)
        .send_and_await_response(30)??;

    Ok(worker_address)
}

static BASE_API_URL: &str = "https://api.telegram.org/bot";

/// function to spawn and initialize a tg bot.
/// call this from your parent process to receive updates!
#[allow(unused)]
pub fn init_tg_bot(
    our: Address,
    token: &str,
    params: Option<GetUpdatesParams>,
) -> anyhow::Result<(Api, Address)> {
    let tg_bot_wasm_path = format!("{}/pkg/tg.wasm", our.package_id());

    // give spawned process both our caps, and grant http_client messaging.
    let our_caps = our_capabilities();
    let http_client = ProcessId::from_str("http_client:distro:sys").unwrap();
    let http_server = ProcessId::from_str("http_server:distro:sys").unwrap();
    let net = ProcessId::from_str("net:distro:sys").unwrap();
    let sqlite = ProcessId::from_str("sqlite:distro:sys").unwrap();
    let barter = ProcessId::from_str("main:barter:appattacc.os").unwrap();

    let process_id = spawn(
        None,
        &tg_bot_wasm_path,
        OnExit::None,
        our_caps,
        vec![http_client, http_server, net, sqlite, barter],
        false,
    )?;
    println!("Spawned");
    let worker_address = Address {
        node: our.node.clone(),
        process: process_id.clone(),
    };

    let api = Api::new(token, our.clone());
    let init = TgInitialize {
        token: token.to_string(),
        params,
    };
    let req = serde_json::to_vec(&TgRequest::RegisterApiKey(init));
    let response = Request::to(worker_address.clone())
        .body(req.unwrap())
        .send_and_await_response(5)??;

    let response_body = response.body();
    // Decode the response body as UTF-8 and print it
    match std::str::from_utf8(response_body) {
        Ok(decoded) => println!("Response from TG worker: {}", decoded),
        Err(e) => println!("Failed to decode response as UTF-8: {}", e),
    }

    Ok((api, worker_address))
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Api {
    pub api_url: String,
    pub our: Address,
    pub current_offset: u32,
}

impl Api {
    #[must_use]
    pub fn new(api_key: &str, our: Address) -> Self {
        let api_url = format!("{BASE_API_URL}{api_key}");
        Self {
            api_url,
            our,
            current_offset: 0,
        }
    }
}

impl TelegramApi for Api {
    type Error = anyhow::Error;

    fn request<T1: serde::ser::Serialize, T2: serde::de::DeserializeOwned>(
        &self,
        method: &str,
        params: Option<T1>,
    ) -> Result<T2, anyhow::Error> {
        let url = format!("{}/{method}", self.api_url);
        let url = url::Url::from_str(&url)?;

        // content-type application/json
        let headers: HashMap<String, String> =
            HashMap::from_iter([("Content-Type".into(), "application/json".into())]);

        let body = if let Some(ref params) = params {
            serde_json::to_vec(params)?
        } else {
            Vec::new()
        };
        let res = send_request_await_response(Method::GET, url, Some(headers), 30, body)?;

        let deserialized: T2 = serde_json::from_slice(&res.body())
            .map_err(|e| anyhow::anyhow!("Failed to deserialize response body: {}", e))?;

        Ok(deserialized)
    }

    fn request_with_form_data<T1: serde::ser::Serialize, T2: serde::de::DeserializeOwned>(
        &self,
        _method: &str,
        _params: T1,
        _files: Vec<(&str, PathBuf)>,
    ) -> Result<T2, anyhow::Error> {
        return Err(anyhow::anyhow!(
            "tgbot doesn't support multipart uploads (yet!)"
        ));
    }
}

impl Api {
    #[allow(unused)]
    pub fn request_no_wait<T1: serde::ser::Serialize>(
        &self,
        method: &str,
        params: Option<T1>,
    ) -> Result<(), anyhow::Error> {
        let url = format!("{}/{method}", self.api_url);
        let url = url::Url::from_str(&url)?;

        // content-type application/json
        let headers: HashMap<String, String> =
            HashMap::from_iter([("Content-Type".into(), "application/json".into())]);

        let body = if let Some(ref params) = params {
            serde_json::to_vec(params)?
        } else {
            Vec::new()
        };
        send_request(Method::GET, url, Some(headers), Some(20), body);
        Ok(())
    }
}
