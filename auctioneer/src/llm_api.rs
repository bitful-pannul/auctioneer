pub fn init_llm(
    our: Address,
    token: &str,
) -> anyhow::Result<()> {
    let llm_api_path = format!("{}/pkg/tg.wasm", our.package_id());

    // give spawned process both our caps, and grant http_client messaging.
    let our_caps = our_capabilities();
    let http_client = ProcessId::from_str("http_client:distro:sys").unwrap();

    let process_id = spawn(
        None,
        &tg_bot_wasm_path,
        OnExit::None,
        our_caps,
        vec![http_client],
        false,
    )?;

    let api = Api::new(token, our.clone());
    let init = TgInitialize {
        token: token.to_string(),
        params,
    };

    let worker_address = Address {
        node: our.node.clone(),
        process: process_id.clone(),
    };

    let _ = Request::new()
        .target(worker_address.clone())
        .body(serde_json::to_vec(&init)?)
        .send();

    Ok((api, worker_address))
}