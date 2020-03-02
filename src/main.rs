#[macro_use]
extern crate lazy_static;

use native_tls::TlsConnector;
use std::collections::HashSet;
use std::collections::HashMap;
use std::hash::Hash;
use std::io::{Read, Write};
use std::net::TcpStream;

use serde::{Deserialize, Serialize};

// Will contain something like "boards.eune.leagueoflegends.com".
static mut BASEURL: String = String::new();
// Will contain something like "hu".
static mut LANGUAGE: String = String::new();
// Will contain something like "EUNE".
static mut REGION: String = String::new();

#[derive(Deserialize, Serialize)]
struct Link {
    description: Option<String>,
    url: Option<String>,
    image: Option<String>
}

#[derive(Deserialize, Serialize)]
struct Thread {
    poster: String,
    date: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    subforum: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    embed: Option<Link>,

    up_votes: usize,
    down_votes: usize,
    body: String,
    replies: Vec<Thread>
}

lazy_static! {
    static ref CPU_COUNT: usize = num_cpus::get();
}

fn make_connection() -> native_tls::TlsStream<TcpStream> {
    let connector = TlsConnector::new().unwrap();
    let stream = unsafe { TcpStream::connect(format!("{}:443", BASEURL)).unwrap() };
    let stream = unsafe {
        connector
            .connect(&BASEURL, stream)
            .unwrap()
    };

    stream
}

fn make_request(request: String) -> String {
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

fn sync_threads<T>(threads: &mut Vec<std::thread::JoinHandle<HashSet<T>>>, results: &mut HashSet<T>)
    where
    T: Eq + Hash,
{
    for t in threads.drain(..) {
        let arr = t.join().unwrap();

        for elem in arr {
            results.insert(elem);
        }
    }
}

fn collect_ids(json: serde_json::Value) -> HashSet<(String, String)> {
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

fn get_user_ids(name: &String) -> HashSet<(String, String)> {
    let initial_response = unsafe { make_request(format!("/{}/player/{}/{}?json_wrap=1", LANGUAGE, REGION, name)) };

    if let Ok(json) = serde_json::from_str(&initial_response) {
        let json: serde_json::Value = json;
        let count = json["searchResultsCount"].as_i64().unwrap();

        let mut results: HashSet<(String, String)> = collect_ids(json);
        let mut threads = Vec::new();

        if count > 50 {
            let rounds = (count / 50) + 1;

            for round in 1..rounds {
                let name = name.clone();
                threads.push(std::thread::spawn(move || {
                    let url = unsafe {
                        format!( "/{}/player/{}/{}?json_wrap=1&num_loaded={}",
                                 LANGUAGE,
                                 REGION,
                                 name,
                                 50 + round * 50)
                    };

                    let response = make_request(url);
                    let json: serde_json::Value = serde_json::from_str(&response).unwrap();

                    collect_ids(json)
                }));

                if threads.len() >= *CPU_COUNT {
                    sync_threads(&mut threads, &mut results);
                }
            }
        }

        sync_threads(&mut threads, &mut results);
        results
    } else {
        print!(" ERROR! Couldn't process {}. ", name);
        HashSet::new()
    }
}

fn download_post(app_id: &String, disc_id: &String) -> serde_json::Value {
    let url = format!("/api/{}/discussions/{}", app_id, disc_id);
    serde_json::from_str(&make_request(url)).unwrap()
}

fn process_raw_thread(root: &serde_json::Value, head: bool, names: &mut HashSet<String>) -> Thread {
    use std::convert::TryInto;

    let mut thread = Thread {
        poster: String::from(
                    root["user"]["name"]
                    .as_str()
                    .unwrap_or("[NEM SIKERÜLT KIOLVASNI]"),
                    ),
                    date: String::from(
                        root["createdAt"]
                        .as_str()
                        .unwrap_or("[NEM SIKERÜLT KIOLVASNI]"),
                        ),
                        up_votes: root["upVotes"].as_u64().unwrap().try_into().unwrap_or(0),
                        down_votes: root["downVotes"].as_u64().unwrap().try_into().unwrap_or(0),
                        replies: Vec::new(),
                        body: {
                            if head {
                                String::from(root["content"]["body"].as_str().unwrap_or("[ÜRES]"))
                            } else {
                                String::from(root["message"].as_str().unwrap_or("[ÜRES]"))
                            }
                        },
                        title: {
                            if head {
                                Some(String::from(
                                        root["title"]
                                        .as_str()
                                        .unwrap_or("[NEM SIKERÜLT KIOLVVASNI]")))
                            } else {
                                None
                            }
                        },
                        subforum: {
                            if head {
                                Some(String::from(
                                        root["application"]["name"]
                                        .as_str()
                                        .unwrap_or("[NEM SIKERÜLT KIOLVVASNI]")))
                            } else {
                                None
                            }
                        },
                        embed: {
                            if let Some(link_root) = root["content"].get("sharedLink") {
                                if let Some(shared_link) = link_root.as_object() {
                                    let description = {
                                        if let Some(node) = shared_link.get("description") {
                                            Some(String::from(node.as_str().unwrap_or("[NINCS LEÍRÁS]")))
                                        } else {
                                            None
                                        }
                                    };

                                    let url = {
                                        if let Some(node) = shared_link.get("url") {
                                            Some(String::from(node.as_str().unwrap_or("[ROSSZ LINK]")))
                                        } else {
                                            None
                                        }
                                    };

                                    let image = {
                                        if let Some(node) = shared_link.get("image") {
                                            Some(String::from(node.as_str().unwrap_or("[NINCS KÉP]")))
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

    if let Some(name) = root["user"]["name"].as_str() {
        names.insert(name.to_string());
    }

    if head {
        if let Some(comments) = root["comments"]["comments"].as_array() {
            for msg in comments {
                thread.replies.push(process_raw_thread(msg, false, names));
            }
        }
    } else {
        if let Some(comments) = root["replies"]["comments"].as_array() {
            for msg in comments {
                thread.replies.push(process_raw_thread(msg, false, names));
            }
        }
    }

    thread
}

fn write_file(dir: &std::path::Path, thread: &Thread, nums: &mut std::collections::HashMap<String, usize>) {
    let path = dir.join(format!("{}/", thread.poster));

    let _ = std::fs::create_dir(&path);

    let mut file = {
        let i = nums.get(&thread.poster).unwrap_or(&0).clone();

        let mut file_path = path.clone();
        file_path.push(format!("{}.json", i));

        let file = std::fs::File::create(file_path).unwrap();
        nums.insert(thread.poster.clone(), i + 1);
        file
    };

    serde_json::to_writer(&mut file, &thread).unwrap();
}

fn process_threads(ids: &HashSet<(String, String)>) -> (Vec<Thread>, HashSet<String>) {
    let count = ids.len();
    let mut done = 0;

    if count > 0 {
        println!("[{} new threads]", count);
    } else {
        println!("[No new threads]");
    }

    let mut threads = Vec::new();
    let mut results = Vec::new();
    let mut names = HashSet::new();

    for (app_id, disc_id) in ids {
        let app_id = app_id.clone();
        let disc_id = disc_id.clone();

        threads.push(std::thread::spawn(move || {
            let mut names = HashSet::new();

            (process_raw_thread(&download_post(&app_id, &disc_id)["discussion"], true, &mut names), names)
        }));

        if threads.len() >= *CPU_COUNT {
            //println!("Catching up...");
            for t in threads.drain(..) {
                if let Ok(retval) = t.join() {
                    results.push(retval.0);
                    names.extend(retval.1);
                } else {
                    println!("Eror unwrapping thread.");
                }
                done += 1;

                if done % 100 == 0 {
                    println!("Done: {}/{}", done, count);
                }
            }
        }
    }

    for t in threads {

        if let Ok(retval) = t.join() {
            results.push(retval.0);
            names.extend(retval.1);
        } else {
            println!("Eror unwrapping thread.");
        }

        done += 1;
        if done % 50 == 0 {
            println!("Done: {}/{}", done, count);
        }
    }

    (results, names)
}

fn add_names(names: HashSet<String>, name: &String, name_queue: &mut HashMap<String, bool>) {
    name_queue.insert(name.to_string(), true);

    let mut names = names.clone();
    names.retain(|elem| {name_queue.contains_key(elem) == false});

    for name in names {
        name_queue.insert(name, false);
    }
}

fn prune_ids(ids: &mut HashSet<(String, String)>, processed_ids: &mut HashSet<(String, String)>) {
    ids.retain(|elem| processed_ids.insert(elem.clone()));
}

fn process_player(name: &String, name_queue: &mut HashMap<String, bool>, processed_ids: &mut HashSet<(String, String)>) -> Vec<Thread> {
    let mut ids = get_user_ids(name);
    prune_ids(&mut ids, processed_ids);
    let (threads, names) = process_threads(&ids);

    add_names(names, name, name_queue);

    threads
}

fn main() {
    println!("This is CHARON, the Boards-backupper.");

    let mut names: HashMap<String, bool> = HashMap::new();
    let mut ids: HashSet<(String, String)> = HashSet::new();
    let mut nums = std::collections::HashMap::new();
    let mut thread_count = 0;

    unsafe {
        println!("Which region do you want to save? [EUNE]");
        let line = {
            let mut line = String::new();
            std::io::stdin().read_line(&mut line).unwrap();
            line.trim_end().to_string()
        };

        if line == "" {
            REGION = String::from("EUNE");
            BASEURL = String::from("boards.eune.leagueoflegends.com");
        } else {
            REGION = line.clone().to_uppercase();
            BASEURL = format!("boards.{}.leagueoflegends.com", line.clone().to_lowercase());
        }

        println!("And which language? [en]");
        let line = {
            let mut line = String::new();
            std::io::stdin().read_line(&mut line).unwrap();
            line.trim_end().to_string()
        };

        if line == "" {
            LANGUAGE = String::from("en");
        } else {
            LANGUAGE = line.clone().to_lowercase();
        }
    }

    println!("Enter a name which will be used to start the process from (Be mindful of capitalization!): ");
    let mut line = {
        let mut line = String::new();
        std::io::stdin().read_line(&mut line).unwrap();
        line.trim_end().to_string()
    };

    while line == "" {
        println!("You must enter a name.");
        line = {
            let mut line = String::new();
            std::io::stdin().read_line(&mut line).unwrap();
            line.trim_end().to_string()
        };
    }

    names.insert(line.clone(), false);

    let dirname = unsafe { format!("./backup_{}_{}", REGION, LANGUAGE) };
    let dir = std::path::Path::new(&dirname);
    let _ = std::fs::create_dir(dir.clone());

    unsafe {
        println!("You are now downloading {}'s posts from the {} region and {} language into {}.", line, REGION, LANGUAGE, dir.display());
    }

    loop {
        if names.iter().all(|(_, val)| {*val}) {break;}

        let (done, all) = {
            let mut done = 0;

            for (_, val) in names.iter() {
                if *val {done+=1;}
            }

            (done, names.len())
        };

        let name: Option<String> = {
            let mut retval = None;
            for (candidate, processed) in names.iter() {
                if !processed {
                    retval = Some(candidate.clone());
                    break;
                }
            }
            retval
        };


        if let Some(name) = name {
            print!("[{}/{} ({}%) ({})] {} ", done, all, (((done as f64)/(all as f64))*100.0) as usize, thread_count, name);
            let threads = process_player(&name, &mut names, &mut ids);

            if threads.len() > 0 {
                for post in threads.iter() {
                    write_file(&dir, post, &mut nums);
                    thread_count += 1;
                }
            }
        }
    }

    println!("Final thread-count: {} threads.", thread_count);
}
