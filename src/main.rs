#[macro_use]
extern crate lazy_static;

use native_tls::TlsConnector;
use std::io::{Read, Write};
use std::net::TcpStream;

fn make_request(request: String) -> String {
    let url = format!("GET {} HTTP/1.0\r\nUser-Agent: Mozilla/4.0 (compatible; MSIE5.01; Windows NT)\r\nHost: boards.eune.leagueoflegends.com\r\n\r\n", request);

    let connector = TlsConnector::new().unwrap();
    let stream = TcpStream::connect("boards.eune.leagueoflegends.com:443").unwrap();
    let mut stream = connector.connect("boards.eune.leagueoflegends.com", stream).unwrap();

    stream.write(url.as_bytes()).unwrap();
    let mut resp = vec![];

    stream.read_to_end(&mut resp).unwrap();

    let resp_full = String::from_utf8(resp).unwrap();
    let split = resp_full.split("\r\n\r\n").map(|val| val.to_string()).collect::<Vec<String>>();

    split[1].clone()
}

fn top_poszt(n: usize) {
    todo!();
}

fn user(nev: String) {
    use regex::Regex;
    lazy_static! {
        static ref REG: Regex = Regex::new(r#"data-application-id="(.*?)" data-discussion-id="(.*?)""#).unwrap();
    }

    let mut results = vec![];
    let mut count = 0;

    loop {
        let url = format!("/hu/player/EUNE/{}?content_type=discussion&json_wrap=1&num_loaded={}", nev, count);
        let response = make_request(url);

        let json: serde_json::Value = serde_json::from_str(&response).unwrap();

        for capture in REG.captures_iter(&json["results"].as_str().unwrap()) {
            println!("{:?} | {:?}", &capture[1], &capture[2]);

            results.push((String::from(&capture[1]), String::from(&capture[2])));
        }

        if count < json["searchResultsCount"].as_i64().unwrap() {
            count += 50;
        } else {
            break;
        }
    }
}

fn poszt_letoltes(app_id: String, disc_id: String) -> serde_json::Value {
    let url = format!("/api/{}/discussions/{}", app_id, disc_id);
    serde_json::from_str(&make_request(url)).unwrap()
}

fn print_thread(root: &serde_json::Value, depth: usize) {
    if depth == 0 {
        println!("{} (+{}/-{}):", root["user"]["name"], root["upVotes"], root["downVotes"]);
        print_thread(&root["comments"], 2);
    }
    else {

        for msg in root["comments"].as_array().unwrap() {
            for i in 0..depth-1 {print!("-");}
            print!("> ");
            println!("{} (+{}/-{})", msg["user"]["name"].as_str().unwrap(), msg["upVotes"], msg["downVotes"]);

            for i in 0..depth {print!(" ");}
            println!("{}", msg["message"]);
            print_thread(&msg["replies"], depth+2);
        }
    }
}

fn main() {
    println!("CHARON vagyok, az alvilág hajósa.\nVálaszd ki mit akarsz tenni:\n\n1 - Top posztok lementése.\n2 - Egy felhasználó posztjainak lementése.\n3 - Egy specifikus poszt letöltése.");

    /*let mut input = String::new();
    std::io::stdin().read_line(&mut input).expect("Bad input!");

    if let Some('\n')=input.chars().next_back() {
        input.pop();
    }
    if let Some('\r')=input.chars().next_back() {
        input.pop();
    }

    println!("{}", input.len());*/

//https://boards.eune.leagueoflegends.com/api/VFnq5EbB/discussions/YhLAqrRM
//
    /*let poszt = poszt_letoltes(String::from("VFnq5EbB"), String::from("LtjfPhOk"));

    print_thread(&poszt["discussion"], 0);*/

    user(String::from("Nemin"));

}
