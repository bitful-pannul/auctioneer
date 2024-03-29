use alloy_signer::LocalWallet;
use kinode_process_lib::{get_state, set_state, Address};
use llm_interface::api::openai::OpenaiApi;
use serde::{Deserialize, Serialize};
use crate::NFTKey;
use crate::tg_api::Api;
use crate::context::ContextManager;

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
pub struct InitialConfig {
    pub openai_key: String,
    pub telegram_bot_api_key: String,
    pub wallet_pk: String,
    pub hosted_url: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct State {
    pub config: InitialConfig,
    pub context_manager: ContextManager,
}

impl State {
    pub fn new(config: InitialConfig) -> Self {
        State {
            config,
            context_manager: ContextManager::new(&[]),
        }
    }

    pub fn fetch() -> Option<State> {
        if let Some(state_bytes) = get_state() {
            bincode::deserialize(&state_bytes).ok()
        } else {
            None
        }
    }

    pub fn save(&self) {
        let serialized_state = bincode::serialize(self).expect("Failed to serialize state");
        set_state(&serialized_state);
    }
}

pub struct Session {
    pub tg_api: Api,
    pub tg_worker: Address,
    pub wallet: LocalWallet,
    pub openai_api: OpenaiApi,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AddNFTArgs {
    pub nft_name: String,
    pub nft_address: String,
    pub nft_id: u64,
    pub chain_id: u64,
    pub nft_description: Option<String>,
    pub sell_prompt: Option<String>,
    pub min_price: String,
}

#[derive(Clone)]
pub enum HttpRequestOutcome {
    Config(InitialConfig),
    AddNFT(AddNFTArgs),
    RemoveNFT(NFTKey),
    None,
}
