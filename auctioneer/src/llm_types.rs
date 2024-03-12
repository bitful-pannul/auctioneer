// TODO: Zen: Upstream this file to llm once this is done.
#[allow(dead_code)]
pub mod openai {
    use serde::Deserialize;
    use serde::Serialize;
    use std::collections::HashMap;

    #[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
    pub enum LLMRequest {
        Embedding(EmbeddingRequest),
        Chat(ChatRequest),
    }

    impl LLMRequest {
        pub fn new_chat_request(api_key: String, params: ChatParams) -> Self {
            let chat_request = ChatRequest {
                api_key,
                params,
            };
            LLMRequest::Chat(chat_request)
        }

        pub fn to_bytes(&self) -> Vec<u8> {
            match self {
                LLMRequest::Chat(_) | LLMRequest::Embedding(_) => serde_json::to_vec(self).unwrap(),
            }
        }

        pub fn parse(bytes: &[u8]) -> Result<LLMRequest, serde_json::Error> {
            serde_json::from_slice::<LLMRequest>(bytes)
        }
    }


    #[derive(Default, Serialize, Deserialize, Debug, Clone, PartialEq)]
    pub struct EmbeddingRequest {
        pub api_key: String,
        pub params: EmbeddingParams,
    }

    #[derive(Default, Serialize, Deserialize, Debug, Clone, PartialEq)]
    pub struct EmbeddingParams {
        pub input: String,
        pub model: String,
    }


    #[derive(Default, Serialize, Deserialize, Debug, Clone, PartialEq)]
    pub struct ChatRequest {
        pub api_key: String,
        pub params: ChatParams,
    }

    #[derive(Default, Serialize, Deserialize, Debug, Clone, PartialEq)]
    pub struct ChatParams {
        pub model: String,
        pub messages: Vec<Message>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub frequency_penalty: Option<f64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub logit_bias: Option<HashMap<String, i32>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub logprobs: Option<bool>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub top_logprobs: Option<i32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub max_tokens: Option<i32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub n: Option<i32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub presence_penalty: Option<f64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub response_format: Option<ResponseFormat>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub seed: Option<i32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub stop: Option<Stop>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub stream: Option<bool>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub temperature: Option<f64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub top_p: Option<f64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub tools: Option<Vec<String>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub tool_choice: Option<ToolChoice>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub user: Option<String>,
    }

    

    #[derive(Default, Serialize, Deserialize, Debug, Clone, PartialEq)]
    pub struct Message {
        pub role: String,
        pub content: String,
    }

    #[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
    #[serde(untagged)]
    pub enum ResponseFormat {
        JsonObject { type_field: String },
    }

    #[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
    #[serde(untagged)]
    pub enum Stop {
        String(String),
        Array(Vec<String>),
    }

    #[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
    #[serde(untagged)]
    pub enum ToolChoice {
        None,
        Auto,
        SpecificFunction {
            type_field: String,
            function: Function,
        },
    }

    #[derive(Default, Serialize, Deserialize, Debug, Clone, PartialEq)]
    pub struct Function {
        pub name: String,
    }

    #[derive(Default, Serialize, Deserialize, Debug, Clone, PartialEq)]
    pub struct ChatResponse {
        pub id: String,
        pub object: String,
        pub created: i64,
        pub model: String,
        pub system_fingerprint: String,
        pub choices: Vec<Choice>,
        pub usage: Usage,
    }

    impl ChatResponse {
        pub fn to_chat_response(&self) -> String {
            self.choices[0].message.content.clone()
        }

        pub fn to_message_response(&self) -> Message {
            self.choices[0].message.clone()
        }
    }

    #[derive(Default, Serialize, Deserialize, Debug, Clone, PartialEq)]
    pub struct Choice {
        pub index: i32,
        pub message: Message,
        pub logprobs: Option<serde_json::Value>,
        pub finish_reason: String,
    }

    #[derive(Default, Serialize, Deserialize, Debug, Clone, PartialEq)]
    pub struct Usage {
        pub prompt_tokens: i32,
        pub completion_tokens: Option<i32>,
        pub total_tokens: i32,
    }

    #[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
    pub enum LLMResponse {
        Embedding(EmbeddingResponse),
        Chat(ChatResponse),
    }

    impl LLMResponse {
        pub fn to_bytes(&self) -> Vec<u8> {
            match self {
                LLMResponse::Chat(_) | LLMResponse::Embedding(_) => {
                    serde_json::to_vec(self).unwrap()
                }
            }
        }

        pub fn parse(bytes: &[u8]) -> Result<LLMResponse, serde_json::Error> {
            serde_json::from_slice::<LLMResponse>(bytes)
        }
    }

    #[derive(Default, Serialize, Deserialize, Debug, Clone, PartialEq)]
    pub struct EmbeddingResponse {
        pub embedding: Vec<f32>,
    }

    impl EmbeddingResponse {
        pub fn from_openai_response(openai_response: OpenAiEmbeddingResponse) -> Self {
            let embedding_values: Vec<f32> = openai_response.data[0]
                .embedding
                .iter()
                .map(|&value| value as f32)
                .collect();
            EmbeddingResponse {
                embedding: embedding_values,
            }
        }
    }

    #[derive(Default, Serialize, Deserialize, Debug, Clone, PartialEq)]
    pub struct OpenAiEmbeddingResponse {
        pub object: String,
        pub data: Vec<EmbeddingData>,
        pub model: String,
        pub usage: Usage,
    }

    #[derive(Default, Serialize, Deserialize, Debug, Clone, PartialEq)]
    pub struct EmbeddingData {
        pub object: String,
        pub index: u32,
        pub embedding: Vec<f64>,
    }
}
