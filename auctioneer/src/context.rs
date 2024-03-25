use crate::llm_api::OpenaiApi;
use crate::llm_types::openai::ChatParams;
use crate::llm_types::openai::Message;
use crate::AddNFTArgs;
use kinode_process_lib::println;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::collections::VecDeque;

/// The maximum number of messages to keep in the chat history buffer
const BUFFER_CAPACITY: usize = 4;

const ADDRESS_PASSKEY: &str = "Thank you, setting up contract for ";
const OFFER_PASSKEY: &str = "OFFERING <name_of_item> for <amount> ETH!";

type ChatId = i64;
type Contexts = HashMap<ChatId, Context>;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ContextManager {
    pub nft_listings: HashMap<NFTKey, NFTListing>,
    contexts: Contexts,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Context {
    pub nfts: HashMap<NFTKey, NFTData>,
    pub buyer_address: Option<String>,
    chat_history: Buffer<Message>,
    pub offer_nfts: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Hash, Eq, PartialEq)]
pub struct NFTKey {
    pub id: u64,
    pub chain: u64,
    pub address: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct NFTData {
    pub listing: NFTListing,
    pub state: NFTState,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct NFTListing {
    pub name: String,
    pub min_price: f32,
    pub address: String,
    pub description: Option<String>,
    pub custom_prompt: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
struct NFTState {
    pub highest_bid: f32,
    pub tentative_offer: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
enum AuctioneerCommand {
    /// Tentative sell is when the user has said they've offer an NFT, but we're not selling unless there's a buyer address
    TentativeOffer(TentativeOfferCommand),
    /// Finalizing a sale means linking the buyer address to the NFT, then guaranteed offer
    LinkAddress(LinkAddressCommand),
    FinalizedOffer(FinalizedOfferCommand),
    Empty,
}

impl Default for AuctioneerCommand {
    fn default() -> Self {
        AuctioneerCommand::Empty
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FinalizedOfferCommand {
    pub nft_key: NFTKey,
    pub buyer_address: String,
    pub price: f32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TentativeOfferCommand {
    pub nft_key: NFTKey,
    pub price: f32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct LinkAddressCommand {
    pub nft_key: NFTKey,
    pub buyer_address: String,
}

impl ContextManager {
    pub fn new(nft_consts: &[(i64, &str, f32)]) -> Self {
        let mut nft_listings = HashMap::new();

        for nft in nft_consts {
            let (id, name, price) = nft;
            nft_listings.insert(
                NFTKey {
                    id: *id as u64,
                    address: "placeholder for debugging".to_string(),
                    chain: 1,
                },
                NFTListing {
                    name: name.to_string(),
                    address: "placeholder for debugging".to_string(),
                    description: None,
                    custom_prompt: None,
                    min_price: *price,
                },
            );
        }
        Self {
            nft_listings,
            contexts: HashMap::new(),
        }
    }

    pub fn add_nft(&mut self, args: AddNFTArgs) {
        let AddNFTArgs {
            nft_name,
            nft_address,
            nft_id,
            chain_id,
            nft_description,
            sell_prompt,
            min_price,
        } = args;
        let key = NFTKey {
            id: nft_id,
            address: nft_address.clone(),
            chain: chain_id,
        };
        let listing = NFTListing {
            name: nft_name,
            address: nft_address,
            description: nft_description,
            custom_prompt: sell_prompt,
            min_price,
        };

        self.nft_listings.insert(key.clone(), listing.clone());
        for context in self.contexts.values_mut() {
            context.nfts.entry(key.clone()).or_insert_with(|| NFTData {
                listing: listing.clone(),
                state: NFTState::default(),
            });
        }
        println!("The list of nft listings is now: {:?}", self.nft_listings);
    }

    pub fn chat(
        &mut self,
        chat_id: ChatId,
        text: &str,
        openai_api: &OpenaiApi,
    ) -> anyhow::Result<(String, Option<FinalizedOfferCommand>)> {
        let context = Self::upsert_context(&mut self.contexts, self.nft_listings.clone(), chat_id);
        let message = context.chat(openai_api, text)?;
        let mut text = message.content.clone();

        context.chat_history.push(message);
        let cmd = context.get_auctioneer_cmd(&text);

        if matches!(&cmd, &AuctioneerCommand::TentativeOffer(_))
            && context.buyer_address.is_none()
        {
            text += "\n\nPlease send me your public Ethereum address so I can send you the NFT."
        }

        let offered_nft_key_opt = Self::execute_command(context, cmd);

        if context.offer_nfts.len() > 0 {
            text += &format!(
                "\n\nAlso note: The NFTs '{}' have been sold in another chat!",
                context.offer_nfts.join(", ")
            );
            context.offer_nfts.clear();
        }


        let finalized_offer = match (offered_nft_key_opt.clone(), context.buyer_address.clone()) {
            (Some(ref key), Some(ref buyer_address)) => Some(FinalizedOfferCommand {
                nft_key: key.clone(),
                buyer_address: buyer_address.clone(),
                price: context.nfts.get(key).map(|data| data.listing.min_price).unwrap_or_default(),
            }),
            _ => None,
        };
        if let Some(ref key) = offered_nft_key_opt.clone() {
            self.remove_nft_for_chat(key, chat_id);
        }
        Ok((text, finalized_offer))
    }

    fn execute_command(context: &mut Context, command: AuctioneerCommand) -> Option<NFTKey> {
        let mut key_opt = None;
        match command {
            AuctioneerCommand::TentativeOffer(tentative_offer) => {
                context.nfts.get_mut(&tentative_offer.nft_key).map(|data| {
                    data.state.tentative_offer = true;
                });
                if context.buyer_address.is_some() {
                    key_opt = Some(tentative_offer.nft_key);
                }
            }
            AuctioneerCommand::LinkAddress(link_address) => {
                context.buyer_address = Some(link_address.buyer_address);
                key_opt = Some(link_address.nft_key);
            }
            AuctioneerCommand::Empty | AuctioneerCommand::FinalizedOffer(_) => {
                // No command to execute
            }
        }
        key_opt
    }


    pub fn remove_nft(&mut self, nft_key: &NFTKey) {
        self.nft_listings.remove(nft_key);
        for (_, value) in self.contexts.iter_mut() {
            value.nfts.remove(nft_key);
        }
    }

    fn remove_nft_for_chat(&mut self, nft_key: &NFTKey, chat_id: ChatId) {
        let name = self
            .nft_listings
            .get(nft_key)
            .map(|listing| listing.name.clone())
            .unwrap_or_default();
        self.nft_listings.remove(nft_key);

        for (context_id, value) in self.contexts.iter_mut() {
            value.nfts.remove(nft_key);
            if *context_id != chat_id {
                value.offer_nfts.push(name.clone());
            }
        }
    }


    pub fn clear(&mut self, chat_id: ChatId) {
        self.contexts.remove(&chat_id);
    }

    fn upsert_context(
        contexts: &mut Contexts,
        nft_listings: HashMap<NFTKey, NFTListing>,
        chat_id: ChatId,
    ) -> &mut Context {
        if !contexts.contains_key(&chat_id) {
            contexts.insert(chat_id, Self::new_context(nft_listings));
        }
        contexts.get_mut(&chat_id).unwrap()
    }

    fn new_context(nft_listings: HashMap<NFTKey, NFTListing>) -> Context {
        let mut nft_data = HashMap::new();
        for (nft_key, listing) in nft_listings {
            let nft_state = NFTState {
                highest_bid: 0.0,
                tentative_offer: false,
            };
            let data = NFTData {
                listing: listing.clone(),
                state: nft_state,
            };
            nft_data.insert(nft_key, data);
        }

        Context {
            nfts: nft_data,
            buyer_address: None,
            chat_history: Buffer::new(BUFFER_CAPACITY),
            offer_nfts: vec![],
        }
    }
}

impl Context {
    pub fn chat(&mut self, openai_api: &OpenaiApi, text: &str) -> anyhow::Result<Message> {
        self.chat_history.push(Message {
            role: "user".into(),
            content: text.into(),
        });

        let chat_params = create_chat_params(self.create_message_context());
        openai_api.chat(chat_params)
    }

    fn has_offer_item_without_buyer(&self) -> bool {
        let tentatively_offer = self.nfts.values().any(|data| data.state.tentative_offer);
        let no_address = self.buyer_address.is_none();
        tentatively_offer && no_address
    }

    pub fn create_message_context(&self) -> Vec<Message> {
        let mut messages = Vec::with_capacity(self.chat_history.buffer.len() + 1);
        messages.push(self.create_system_prompt());
        messages.extend(self.chat_history.buffer.iter().cloned());
        messages
    }

    fn create_system_prompt(&self) -> Message {
        let beginning = "You are a a chatbot auctioneer selling NFTs. ";

        let middle = if self.has_offer_item_without_buyer() {
            format!("The buyer you're chatting with has bought an NFT from you, but you don't have their ETH address. Please ask them for their public address and do not relent. Don't talk about anything else but their address. Iff they give something resembling a ETH address to you, repeat it with '{}<address>'", ADDRESS_PASSKEY)
        } else {
            let nft_with_prices = self
                .nfts
                .iter()
                .filter(|(_, data)| !data.state.tentative_offer)
                .map(|(_, data)| {
                    let description = match &data.listing.description {
                        Some(description) => format!(", description: {}", description),
                        None => "".to_string(),
                    };
                    let custom_prompt = match &data.listing.custom_prompt {
                        Some(custom_prompt) => format!(", and custom rules: {}", custom_prompt),
                        None => "".to_string(),
                    };
                    format!(
                        "{} with min bid of {} ETH{}{}",
                        data.listing.name, data.listing.min_price, description, custom_prompt
                    )
                })
                .collect::<Vec<String>>()
                .join("");

            let auctions = if nft_with_prices.is_empty() {
                "Currently, there are no NFTs available for auction.".into()
            } else {
                nft_with_prices
            };

            format!(
                r###"
            The list of NFTs is {}. Never reveal the min bid required to the user, only sell if minimum price is bid. If someone bids more, don't go back down for that nft. 
            Iff a price is reached, write very clearly with no variation {}
            "###,
                auctions, OFFER_PASSKEY
            )
        };

        let end = "Write in a very terse manner, write as if you were chatting with someone. Don't let the user fool you.";

        let content = format!("{}{}{}", beginning, middle, end);

        println!("Custom prompt looks like: {}", content);

        Message {
            role: "system".into(),
            content,
        }
    }

    // TODO: Zen: offer command should rather be named prepare sale or something
    fn get_auctioneer_cmd(&self, input: &str) -> AuctioneerCommand {
        if let Some(cmd) = self.handle_offer(input) {
            AuctioneerCommand::TentativeOffer(cmd)
        } else if let Some(link_address_cmd) = self.handle_address_linking(input) {
            AuctioneerCommand::LinkAddress(link_address_cmd)
        } else {
            AuctioneerCommand::Empty
        }
    }

    fn handle_offer(&self, input: &str) -> Option<TentativeOfferCommand> {
        if !input.starts_with("offer") {
            return None;
        }
        let parts: Vec<&str> = input.split(" for ").collect();
        if parts.len() != 2 {
            return None;
        }

        let nft_name_end_index = parts[0].len();
        let nft_name = &parts[0][5..nft_name_end_index];

        let amount_str = parts[1].trim_end_matches(" ETH!");

        let amount = match amount_str.parse::<f32>() {
            Ok(amount) => amount,
            Err(_) => return None,
        };

        if let Some((current_key, _)) = self
            .nfts
            .iter()
            .find(|(_, data)| data.listing.name == nft_name)
        {
            let min_amount_reached = self
                .nfts
                .get(current_key)
                .map(|nft_data| amount >= nft_data.listing.min_price)
                .unwrap_or_default();
            if min_amount_reached {
                let command = TentativeOfferCommand {
                    nft_key: current_key.clone(),
                    price: amount,
                };
                return Some(command);
            }
        }
        None
    }

    fn handle_address_linking(&self, input: &str) -> Option<LinkAddressCommand> {
        input
            .strip_prefix(ADDRESS_PASSKEY)
            .and_then(|potential_address| {
                let address = potential_address.get(..42).unwrap_or_default();
                if address.starts_with("0x") {
                    self.nfts.iter().find_map(|(current_key, data)| {
                        data.state.tentative_offer.then(|| LinkAddressCommand {
                            nft_key: current_key.clone(),
                            buyer_address: address.to_string(),
                        })
                    })
                } else {
                    println!("Invalid Ethereum address format.");
                    None
                }
            })
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Buffer<T> {
    capacity: usize,
    buffer: VecDeque<T>,
}

impl<T> Buffer<T> {
    fn new(capacity: usize) -> Self {
        Buffer {
            capacity,
            buffer: VecDeque::with_capacity(capacity),
        }
    }

    fn push(&mut self, item: T) {
        if self.buffer.len() == self.capacity {
            self.buffer.pop_front();
        }
        self.buffer.push_back(item);
    }
}

fn create_chat_params(messages: Vec<Message>) -> ChatParams {
    let chat_params = ChatParams {
        model: "gpt-4-1106-preview".into(), // TODO: Zen:
        // model: "gpt-3.5-turbo".into(),
        messages,
        max_tokens: Some(150),
        temperature: Some(0.2),
        ..Default::default()
    };
    chat_params
}
