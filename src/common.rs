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

fn make_connection() -> native_tls::TlsStream<TcpStream> {
    let connector = TlsConnector::new().unwrap();
    let stream = unsafe { TcpStream::connect(format!("{}:443", BASEURL)).unwrap() };
    let stream = unsafe {
        match connector.connect(&BASEURL, stream) {
            Ok(stream) => stream,
            Err(_) => {
                println!("Error connecting, retrying");

                let connector = TlsConnector::new().unwrap();
                let stream = TcpStream::connect(format!("{}:443", BASEURL)).unwrap();
                connector.connect(&BASEURL, stream).unwrap()
            }
        }
    };

    stream
}

pub fn make_request(request: String) -> String {
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

    split[1].clone()
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
