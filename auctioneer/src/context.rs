use crate::llm_api::OpenaiApi;
use crate::llm_types::openai::ChatParams;
use crate::llm_types::openai::Message;
use crate::AddNFTArgs;
use kinode_process_lib::eth;
use kinode_process_lib::println;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::collections::VecDeque;

/// The maximum number of messages to keep in the chat history buffer
const BUFFER_CAPACITY: usize = 4;

const ADDRESS_PASSKEY: &str = "Thank you, setting up contract for ";
const SELLING_PASSKEY: &str = "OFFER <name_of_item> for <amount> ETH!";

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
    TentativeOffer(OfferCommand),
    /// Finalizing a sale means linking the buyer address to the NFT, then guaranteed offer
    FinalizeOffer(OfferCommand),
    Empty,
}

impl Default for AuctioneerCommand {
    fn default() -> Self {
        AuctioneerCommand::Empty
    }
}

// pls duplicate offercommand to tentativveoffecommand
// internal command to be used by the auctioneer
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct OfferCommand {
    pub nft_key: NFTKey,
    pub buyer_address: Option<String>,
    pub price: f32,
}

impl ContextManager {
    pub fn new(nft_consts: &[(i64, &str, f32); 3]) -> Self {
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
    ) -> anyhow::Result<(String, Option<OfferCommand>)> {
        let context = Self::upsert_context(&mut self.contexts, self.nft_listings.clone(), chat_id);
        let message = context.chat(openai_api, text)?;
        let mut text = message.content.clone();

        context.chat_history.push(message);
        let command = context.auctioneer_command(&text);

        if matches!(&command, &AuctioneerCommand::TentativeOffer(_))
            && context.buyer_address.is_none()
        {
            text += "\n\nPlease send me your public Ethereum address so I can send you the NFT."
        }

        let offer = Self::execute_command(context, command);

        if context.offer_nfts.len() > 0 {
            text += &format!(
                "\n\nAlso note: The NFTs '{}' have been offer in another chat!",
                context.offer_nfts.join(", ")
            );
            context.offer_nfts.clear();
        }

        if let Some(ref offer) = offer {
            self.remove_nft_for_chat(&offer.nft_key, chat_id);
        }

        Ok((text, offer))
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

    // TODO: I don't know whether we actually need this? We just need a way to upstream all the way to lib for the sale...
    fn execute_command(context: &mut Context, command: AuctioneerCommand) -> Option<OfferCommand> {
        let mut offer_command = None;
        match command {
            AuctioneerCommand::TentativeOffer(sale) => {
                context.nfts.get_mut(&sale.nft_key).map(|data| {
                    data.state.tentative_offer = true;
                });
                if context.buyer_address.is_some() {
                    offer_command = Some(sale);
                }
            }
            AuctioneerCommand::FinalizeOffer(sale) => {
                context.buyer_address = sale.buyer_address.clone();
                offer_command = Some(sale);
            }
            AuctioneerCommand::Empty => {
                // No command to execute
            }
        }
        offer_command
    }

    pub fn clear(&mut self, chat_id: ChatId) {
        self.contexts.remove(&chat_id);
    }

    pub fn get_message_history(&self, chat_id: ChatId) -> Vec<Message> {
        self.contexts
            .get(&chat_id)
            .unwrap()
            .create_message_context()
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
                auctions, SELLING_PASSKEY
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
    fn auctioneer_command(&self, input: &str) -> AuctioneerCommand {
        if let Some(command) = self.handle_offer(input) {
            AuctioneerCommand::TentativeOffer(command)
        } else if let Some(offer_command) = self.handle_address_linking(input) {
            AuctioneerCommand::FinalizeOffer(offer_command)
        } else {
            AuctioneerCommand::Empty
        }
    }

    fn handle_offer(&self, input: &str) -> Option<OfferCommand> {
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
                let command = OfferCommand {
                    nft_key: current_key.clone(),
                    buyer_address: self.buyer_address.clone(),
                    price: amount,
                };
                return Some(command);
            }
        }
        None
    }

    // todo jaxs: cleanup
    fn handle_address_linking(&self, input: &str) -> Option<OfferCommand> {
        if input.starts_with(ADDRESS_PASSKEY) {
            let potential_address = input.strip_prefix(ADDRESS_PASSKEY).unwrap_or_default();
            let address = &potential_address[..42];
            if address.starts_with("0x") {
                let buyer_address = address.to_string();
                let offer_command = self.nfts.iter().find_map(|(current_key, data)| {
                    if data.state.tentative_offer {
                        Some(OfferCommand {
                            nft_key: current_key.clone(),
                            price: data.state.highest_bid,
                            buyer_address: Some(buyer_address.clone()),
                        })
                    } else {
                        None
                    }
                });
                return offer_command;
            } else {
                println!("Provided address does not follow the Ethereum address format.");
            }
        }
        None
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
