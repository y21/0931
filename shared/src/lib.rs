use serde::Deserialize;
use serde::Serialize;

#[derive(Deserialize, Serialize, Debug)]
pub enum ClientMessage {
    EvalResponse(Result<String, String>),
}

#[derive(Deserialize, Serialize)]
pub enum HostMessage {
    Eval(String),
}
