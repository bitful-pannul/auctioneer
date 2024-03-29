use alloy_primitives::{Address as EthAddress, FixedBytes};
use alloy_signer::LocalWallet;
use kinode_process_lib::{
    eth, println, Address, 
};
use llm_interface::api::openai::spawn_openai_pkg;
use std::str::FromStr;
use crate::context::ContextManager;
use crate::tg_api::init_tg_bot;
use crate::State;
use crate::InitialConfig;

pub fn hydrate_state(our: &Address, config: InitialConfig, context_manager: ContextManager) -> anyhow::Result<State> {
    let Ok(openai_api) = spawn_openai_pkg(our.clone(), &config.openai_key) else {
        return Err(anyhow::anyhow!("openAI couldn't boot."));
    };
    let Ok((tg_api, tg_worker)) =
        init_tg_bot(our.clone(), &config.telegram_bot_api_key, None)
    else {
        return Err(anyhow::anyhow!("tg bot couldn't boot."));
    };

    let Ok(wallet) = config.wallet_pk.parse::<LocalWallet>() else {
        return Err(anyhow::anyhow!("couldn't parse private key."));
    };

    // subscribe to updates...
    let escrow_address =
        EthAddress::from_str("0x4A3A2c0A385F017501544DcD9C6Eb3f6C63fc38b").unwrap();

    let mut seller_topic_bytes = [0u8; 32];
    seller_topic_bytes[12..].copy_from_slice(&wallet.address().to_vec());
    let seller_topic: FixedBytes<32> = FixedBytes::from_slice(&seller_topic_bytes);

    let filter = eth::Filter::new()
        .address(escrow_address)
        .from_block(0)
        .to_block(eth::BlockNumberOrTag::Latest)
        .events(vec![
            "NFTPurchased(address,address,uint256,address,uint256)",
        ])
        .topic1(seller_topic);

    let sep = eth::Provider::new(11155111, 15);
    let op = eth::Provider::new(10, 15);
    let base = eth::Provider::new(8453, 15);
    let arb = eth::Provider::new(42161, 15);

    if let Err(e) = sep.subscribe(1, filter.clone()) {
        println!("Failed to subscribe to sep: {:?}", e);
    }
    if let Err(e) = op.subscribe(2, filter.clone()) {
        println!("Failed to subscribe to op: {:?}", e);
    }
    if let Err(e) = base.subscribe(3, filter.clone()) {
        println!("Failed to subscribe to base: {:?}", e);
    }
    if let Err(e) = arb.subscribe(4, filter) {
        println!("Failed to subscribe to arb: {:?}", e);
    }

    Ok(State {
        our: our.clone(),
        config, 
        context_manager,
        tg_api,
        tg_worker,
        wallet,
        openai_api,
    })
}