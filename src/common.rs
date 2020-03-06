use native_tls::TlsConnector;
use std::net::TcpStream;
use std::io::{Read, Write};
use std::collections::HashSet;

// Will contain something like "boards.eune.leagueoflegends.com".
pub static mut BASEURL: String = String::new();
// Will contain something like "hu".
pub static mut LANGUAGE: String = String::new();
// Will contain something like "EUNE".
pub static mut REGION: String = String::new();

use crate::thread::*;

fn repeat_connection(depth: usize) -> native_tls::TlsStream<TcpStream> {
    if depth > 0 {
        let connector = TlsConnector::new().unwrap();
        unsafe {
            match TcpStream::connect(format!("{}:443", BASEURL)) {
                Ok(stream) =>
                    match connector.connect(&BASEURL, stream) {
                        Ok(stream) => stream,
                        Err(_) =>  {
                            println!("Error connecting, attempt {}/5", 5-depth);
                            repeat_connection(depth-1)
                        }
                    },
                Err(_) => {
                    println!("Error connecting, attempt {}/5.", 5-depth);
                    repeat_connection(depth-1)
                }
            }
        }
    } else {
        panic!("Ran out of retry attempts!")
    }
}

fn make_connection() -> native_tls::TlsStream<TcpStream> {
    repeat_connection(5)
}

fn repeat_request(request: String, depth: usize) -> String {
    use regex::Regex;

    lazy_static! {
        static ref OK: Regex = Regex::new("HTTP/1.1 200 OK").unwrap();
    }

    if depth > 0 {
        let url = unsafe { format!("GET {} HTTP/1.0\r\nUser-Agent: Mozilla/4.0 (compatible; MSIE5.01; Windows NT)\r\nHost: {}\r\n\r\n", request, BASEURL) };
        let mut stream = make_connection();

        stream.write(url.as_bytes()).unwrap();
        let mut resp = vec![];

        stream.read_to_end(&mut resp).unwrap();

        let resp_full = String::from_utf8(resp).unwrap();
        let split = resp_full
            .split("\r\n\r\n")
            .map(|val| val.to_string())
            .collect::<Vec<String>>();

        if OK.is_match(&split[0]) {
            split[1].clone()
        } else {
            println!("Error making request, attempt {}/5.", 5-depth);
            repeat_request(request, depth-1)
        }
    } else {
        panic!("Ran out of request attempts!");
    }
}

pub fn make_request(request: String) -> String {
    repeat_request(request, 5)
}

pub fn download_post_by_id(app_id: &String, disc_id: &String) -> serde_json::Value {
    let url = format!("/api/{}/discussions/{}", app_id, disc_id);
    serde_json::from_str(&make_request(url)).unwrap()
}

pub fn collect_ids(json: serde_json::Value) -> HashSet<(String, String)> {
    use regex::Regex;
    lazy_static! {
        static ref REG: Regex =
            Regex::new(r#"data-application-id="(.*?)" data-discussion-id="(.*?)""#).unwrap();
    }

    let mut results: HashSet<(String, String)> = HashSet::new();

    if let Some(json_results) = json["results"].as_str() {
        for capture in REG.captures_iter(&json_results) {
            let tuple: (String, String) = (String::from(&capture[1]), String::from(&capture[2]));
            results.insert(tuple);
        }
    }

    if let Some(json_results) = json["discussions"].as_str() {
        for capture in REG.captures_iter(&json_results) {
            let tuple: (String, String) = (String::from(&capture[1]), String::from(&capture[2]));
            results.insert(tuple);
        }
    }

    results
}

pub fn process_raw_thread(root: &serde_json::Value, head: bool) -> Thread {
    use std::convert::TryInto;

    let mut thread = Thread {
        poster: String::from(
                    root["user"]["name"]
                    .as_str()
                    .unwrap_or("[COULD NOT READ POSTER]"),
                    ),
                    date: String::from(
                        root["createdAt"]
                        .as_str()
                        .unwrap_or("[COULDN'T READ DATE]"),
                        ),
                        up_votes: root["upVotes"].as_u64().unwrap_or(0).try_into().unwrap_or(0),
                        down_votes: root["downVotes"].as_u64().unwrap_or(0).try_into().unwrap_or(0),
                        replies: Vec::new(),
                        body: {
                            if head {
                                String::from(root["content"]["body"].as_str().unwrap_or("[BODY IS EMPTY]"))
                            } else {
                                String::from(root["message"].as_str().unwrap_or("[BODY IS EMPTY]"))
                            }
                        },
                        title: {
                            if head {
                                Some(String::from(
                                        root["title"]
                                        .as_str()
                                        .unwrap_or("[COULDN'T READ TITLE]")))
                            } else {
                                None
                            }
                        },
                        subforum: {
                            if head {
                                Some(String::from(
                                        root["application"]["name"]
                                        .as_str()
                                        .unwrap_or("[COULDN'T READ SUBFORUM]"))) } else { None
                            }
                        },
                        embed: {
                            if let Some(link_root) = root["content"].get("sharedLink") {
                                if let Some(shared_link) = link_root.as_object() {
                                    let description = {
                                        if let Some(node) = shared_link.get("description") {
                                            Some(String::from(node.as_str().unwrap_or("[NO DESCRIPTION]")))
                                        } else {
                                            None
                                        }
                                    };

                                    let url = {
                                        if let Some(node) = shared_link.get("url") {
                                            Some(String::from(node.as_str().unwrap_or("[COULDN'T READ LINK]")))
                                        } else {
                                            None
                                        }
                                    };

                                    let image = {
                                        if let Some(node) = shared_link.get("image") {
                                            Some(String::from(node.as_str().unwrap_or("[COULDN'T READ IMAGE LINK]")))
                                        } else {
                                            None
                                        }
                                    };

                                    Some(Link { description, url, image })
                                } else {
                                    None
                                }
                            } else {
                                None
                            }
                        }
    };

    if head {
        if let Some(comments) = root["comments"]["comments"].as_array() {
            for msg in comments {
                thread.replies.push(process_raw_thread(msg, false));
            }
        }
    } else {
        if let Some(comments) = root["replies"]["comments"].as_array() {
            for msg in comments {
                thread.replies.push(process_raw_thread(msg, false));
            }
        }
    }

    thread
}

