#[macro_use]
extern crate lazy_static;

use bytes::buf::ext::BufExt as _;
use hyper_tls::HttpsConnector;
use hyper::Client;
//use hyper::body::HttpBody as _;
//use tokio::io::{self, AsyncWriteExt as _};
use serde::{Deserialize, Serialize};

use std::collections::{HashSet, HashMap};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;
//type TlsClient = hyper::Client<hyper_tls::HttpsConnector<hyper::client::HttpConnector>>;

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


async fn fetch_json(url: String) -> Result<serde_json::Value> {
    let https = HttpsConnector::new();
    let client = Client::builder().build::<_, hyper::Body>(https);
    let url = format!("https://boards.eune.leagueoflegends.com{}", url);

    let req = hyper::Request::builder()
	.method("GET")
	.uri(url)
	.header("User-Agent", "Mozilla/4.0 (compatible; MSIE5.01; Windows NT)")
	.header("Host", "boards.eune.leagueoflegends.com")
	.body(hyper::Body::from(""))?;

    let res = client.request(req).await?;
    let body = hyper::body::aggregate(res).await?;
    let request = serde_json::from_reader(body.reader())?;

    Ok(request)
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

async fn get_user_ids(name: &String) -> Result<HashSet<(String, String)>> {
    let name = percent_encoding::utf8_percent_encode(name, percent_encoding::NON_ALPHANUMERIC);
    
    let json = fetch_json(format!("/hu/player/EUNE/{}?json_wrap=1", name)).await?;
    let count = json["searchResultsCount"].as_i64().unwrap();

    let mut results: HashSet<(String, String)> = collect_ids(json);

    if count > 50 {
	let rounds = (count / 50) + 1;
	let mut threads = vec![];
	
	for round in 1..rounds {
	    let json = fetch_json(format!("/hu/player/EUNE/{}?json_wrap=1&num_loaded={}",
					  name,
					  50 + round*50));
	    threads.push(json);
	}

	let threads = futures::future::join_all(threads).await;
	for t in threads {
	    if let Ok(result) = t {
		results.extend(collect_ids(result));
	    } else {
		println!("Couldn't unwrap IDs for {}.", name);
	    }
	}
    }

    Ok(results)
}

async fn download_threads(ids: &HashSet<(String, String)>) -> Result<(Vec<Thread>, HashSet<String>)> {
    let count = ids.len();

    let mut threads = vec![];
    let mut results = vec![];
    let mut names = HashSet::new();
    for (app, disc) in ids.iter() {
	threads.push(fetch_json(format!("/api/{}/discussions/{}", app, disc)));
    }

    println!("Doing {} threads.", count);
    let jsons = futures::future::join_all(threads).await;

    for json in jsons {
	if let Ok(json) = json {
	    results.push(process_raw_thread(&json["discussion"], true, &mut names));
	} else {
	    println!("Couldn't download a thread.");
	}
    }
    
    Ok((results, names))
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

async fn process_profile(name: &String,
			 name_queue: &mut HashMap<String, bool>,
			 processed_ids: &mut HashSet<(String, String)>) -> Result<Vec<Thread>> {
    println!("Processing {}", name);
    let mut ids = get_user_ids(name).await?;
    prune_ids(&mut ids, processed_ids);
    let (threads, names) = download_threads(&ids).await?;
    add_names(names, &name, name_queue);

    Ok(threads)
}

fn write_file(thread: &Thread, nums: &mut std::collections::HashMap<String, usize>) {
    let dir = "../posztok";
    let _ = std::fs::create_dir(dir);
    let _ = std::fs::create_dir(format!("{}/{}", dir, thread.poster));

    let mut file = {
        if let Some(i) = nums.remove(&thread.poster) {
            let file =
                std::fs::File::create(&format!("{}/{}/{}.json", dir, thread.poster, i)).unwrap();
            nums.insert(thread.poster.clone(), i + 1);
            file
        } else {
            let file =
                std::fs::File::create(&format!("{}/{}/{}.json", dir, thread.poster, 0)).unwrap();
            nums.insert(thread.poster.clone(), 1);
            file
        }
    };

    serde_json::to_writer(&mut file, &thread).unwrap();
}

#[tokio::main]
async fn main() -> Result<()> {
    let mut names: HashMap<String, bool> = HashMap::new();
    let mut ids: HashSet<(String, String)> = HashSet::new();
    names.insert(String::from("Nemin"), false);
    
    let mut thread_count: usize = 0;
    let mut nums = HashMap::new();

    loop {
	if names.iter().all(|(_, val)| {*val}) {break;}

	let all = names.len();
	let mut done: usize = 0;
	for (_, val) in names.iter() {
	    if *val {done+=1;}
	}

	for (candidate, processed) in names.clone().iter() {
	    if !processed {
		let name = candidate;
		print!("[{}/{} ({}%) ({})] {} ",
		       done, all, (((done as f64)/(all as f64))*100.0) as usize, thread_count, name);

		let threads = process_profile(&name, &mut names, &mut ids).await?;
		if threads.len() > 0 {
		    for post in threads.iter() {
			write_file(post, &mut nums);
			thread_count += 1;
		    }
		}
	    }
	}
    }

    println!("Final thread-count: {} threads.", thread_count);
    Ok(())
}
