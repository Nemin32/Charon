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

fn collect_ids(json: serde_json::Value) -> Vec<(String, String)> {
    use regex::Regex;
    lazy_static! {
        static ref REG: Regex = Regex::new(r#"data-application-id="(.*?)" data-discussion-id="(.*?)""#).unwrap();
    }

    let mut results: Vec<(String, String)> = vec![];

    for capture in REG.captures_iter(&json["results"].as_str().unwrap()) {

        let tuple: (String, String) = (String::from(&capture[1]), String::from(&capture[2]));
        if !results.iter().any(|elem| elem.0 == tuple.0 && elem.1 == tuple.1) {
            results.push(tuple);
        }
    }

    results
}

fn get_user_thread_ids(nev: String) -> Vec<(String, String)> {


    let initial_response = make_request(format!("/hu/player/EUNE/{}?json_wrap=1", nev));
    let json: serde_json::Value = serde_json::from_str(&initial_response).unwrap();
    let count = json["searchResultsCount"].as_i64().unwrap();

    let mut results: Vec<(String, String)> = collect_ids(json);

    let mut threads = Vec::new();

    if count > 50 {
        let rounds = (count/50)+1;

        for round in 1..rounds {
            let nev = nev.clone();
            threads.push(std::thread::spawn(move || {
                let url = format!("/hu/player/EUNE/{}?json_wrap=1&num_loaded={}", nev, 50 + round*50);
                println!("{}", url);

                let response = make_request(url);
                let json: serde_json::Value = serde_json::from_str(&response).unwrap();

                collect_ids(json)
            }));
        }
    }

    for t in threads {
        let arr = t.join().unwrap();
        results.extend(arr.into_iter());
    }

    println!("Finished collecting IDs.");

    results
}

fn download_post(app_id: &String, disc_id: &String) -> serde_json::Value {
    let url = format!("/api/{}/discussions/{}", app_id, disc_id);
    serde_json::from_str(&make_request(url)).unwrap()
}

fn print_thread(root: &serde_json::Value, depth: usize) -> Thread {
    use std::convert::TryInto;

    let mut head = Thread {
    poster    : String::from(root["user"]["name"].as_str().unwrap_or("[NEM SIKERÜLT KIOLVASNI]")),
    date      : String::from(root["createdAt"].as_str().unwrap_or("[NEM SIKERÜLT KIOLVASNI]")),
    upVotes   : root["downVotes"].as_u64().unwrap().try_into().unwrap(),
    downVotes : root["upVotes"].as_u64().unwrap().try_into().unwrap(),
    replies   : Vec::new(),
    body      :
        {
            if depth == 0 {
                String::from(root["content"]["body"].as_str().unwrap_or("[ÜRES]"))
            } else {
                String::from(root["message"].as_str().unwrap_or("[ÜRES]"))
            }
        }
    };

    if depth == 0 {
        if let Some(comments) = root["comments"]["comments"].as_array() {
            for msg in comments {
                head.replies.push(print_thread(msg, depth+2));
            }
        }
    } else {
        if let Some(comments) = root["replies"]["comments"].as_array() {
            for msg in comments {
                head.replies.push(print_thread(msg, depth+2));
            }
        }
    }

    head
}


use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
struct Thread {
    poster: String,
    date: String,
    upVotes: usize,
    downVotes: usize,
    body: String,
    replies: Vec<Thread>
}

fn main() {
    use std::fs;

    println!("CHARON vagyok, az alvilág hajósa.\nVálaszd ki mit akarsz tenni:\n\n1 - Top posztok lementése.\n2 - Egy felhasználó posztjainak lementése.\n3 - Egy specifikus poszt letöltése.");

    let mut input = String::new();
    std::io::stdin().read_line(&mut input).expect("Bad input!");

    if let Some('\n')=input.chars().next_back() {
        input.pop();
    }
    if let Some('\r')=input.chars().next_back() {
        input.pop();
    }

//https://boards.eune.leagueoflegends.com/api/VFnq5EbB/discussions/YhLAqrRM

    let _ = std::fs::create_dir("posztok");

    //let username = "Mind The Gap";

    let username = &input;

    let ids = get_user_thread_ids(String::from(username));
    let _ = std::fs::create_dir(format!("./posztok/{}", username));



    let mut threads = Vec::new();

    for (i, (app_id, disc_id)) in ids.iter().enumerate() {
        let app_id = app_id.clone();
        let disc_id = disc_id.clone();
        let username = username.clone();

        threads.push(std::thread::spawn(move || {
        let poszt = download_post(&app_id, &disc_id);

        println!("Kezdés: {}", poszt["discussion"]["title"]);

        let mut file = std::fs::File::create(&format!("./posztok/{}/{}.txt", username, i)).unwrap();
        let thread = print_thread(&poszt["discussion"], 0);

        serde_json::to_writer(&mut file, &thread).unwrap();

        println!("Vég: {}", poszt["discussion"]["title"]);
        }));

        if threads.len() > 12 {
            println!("Catching up...");
            for t in threads {
                t.join().unwrap();
            }
            threads = Vec::new();
        }
    }

    //let mut posztok = Vec::new();

    for t in threads {
        //posztok.push(t.join());
        //
        t.join().unwrap();
    }

    //let poszt = download_post(&String::from("VFnq5EbB"), &String::from("LtjfPhOk"));



    //let posts = get_user_thread_ids(String::from("Nemin"));

    //let letoltes = download_post(&posts[0].0, &posts[0].1);

    //print_thread(&letoltes["discussion"], 0, &mut file);


}
