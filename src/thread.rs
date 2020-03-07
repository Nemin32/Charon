use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
pub struct Link {
    pub title: Option<String>,
    pub description: Option<String>,
    pub url: Option<String>,
    pub image: Option<String>
}

#[derive(Deserialize, Serialize)]
pub struct Thread {
    pub poster: String,
    pub date: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subforum: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub embed: Option<Link>,

    pub up_votes: usize,
    pub down_votes: usize,
    pub body: String,
    pub replies: Vec<Thread>
}

