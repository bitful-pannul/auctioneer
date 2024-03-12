
use kinode_process_lib::{
    http::{send_request, send_request_await_response, Method},
    our_capabilities, spawn, Address, OnExit, ProcessId, Request, println
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::{path::PathBuf, str::FromStr};

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
