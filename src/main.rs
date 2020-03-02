#[macro_use]
extern crate lazy_static;

use std::collections::HashSet;
use std::collections::HashMap;
use std::hash::Hash;

pub mod common;
use crate::common::*;

pub mod redtracker;
use crate::redtracker::*;

pub mod thread;
use crate::thread::*;

lazy_static! {
    static ref CPU_COUNT: usize = num_cpus::get();
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


fn get_user_ids(name: String) -> HashSet<(String, String)> {
    let initial_response = make_request(name.clone());

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
                    let url = format!("{}&num_loaded={}",
                                      name,
                                      50 + round * 50);

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

            (process_raw_thread(&download_post_by_id(&app_id, &disc_id)["discussion"], true, &mut names), names)
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

fn add_names(mut names: HashSet<String>, name: &String, name_queue: &mut HashMap<String, bool>) {
    name_queue.insert(name.to_string(), true);
    names.retain(|elem| {name_queue.contains_key(elem) == false});

    for name in names {
        name_queue.insert(name, false);
    }
}

fn write_names(dir: &std::path::Path, name_queue: &HashMap<String, bool>) {
    use std::io::Write;

    let name1 = dir.clone().join("unprocessed.txt");
    let name2 = dir.clone().join("processed.txt");

    let mut unp = std::fs::File::create(name1).unwrap();
    let mut p = std::fs::File::create(name2).unwrap();

    for (name, processed) in name_queue {
        if !processed {
            let _ = write!(unp, "{}\n", name);
        } else {
            let _ = write!(p, "{}\n", name);
        }
    }
}

fn prune_ids(ids: &mut HashSet<(String, String)>, processed_ids: &mut HashSet<(String, String)>) {
    ids.retain(|elem| processed_ids.insert(elem.clone()));
}

fn process_player(name: &String, name_queue: &mut HashMap<String, bool>, processed_ids: &mut HashSet<(String, String)>) -> Vec<Thread> {
    let name_request = unsafe { format!("/{}/player/{}/{}?json_wrap=1", LANGUAGE, REGION, name) };
    let mut ids = get_user_ids(name_request);
    prune_ids(&mut ids, processed_ids);
    let (threads, names) = process_threads(&ids);

    add_names(names, name, name_queue);

    threads
}

fn load_names(dir: &std::path::Path) -> HashMap<String, bool> {
    let mut names = HashMap::new();

    let unp = dir.clone().join("unprocessed.txt");
    let p = dir.clone().join("processed.txt");

    if let Ok(name_string) = std::fs::read_to_string(&unp) {
        for line in name_string.lines() {
            names.insert(line.to_string(), false);
        }
    }

    if let Ok(name_string) = std::fs::read_to_string(&p) {
        for line in name_string.lines() {
            names.insert(line.to_string(), true);
        }
    }

    /*if let Ok(entries) = std::fs::read_dir(&dir) {
        for folder in entries {
            if let Ok(folder) = folder {
                let name = folder.file_name().into_string();

                if let Ok(name) = name {
                    names.insert(name, true);
                }
            }
        }
    }*/

    names
}

fn handle_names(dir: &std::path::Path, nums: &mut HashMap<String, usize>, names: &mut HashMap<String, bool>, processed_ids: &mut HashSet<(String, String)>) {
    let mut thread_count = 0;

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
            let threads = process_player(&name, names, processed_ids);

            if threads.len() > 0 {
                for post in threads.iter() {
                    write_file(&dir, post, nums);
                    thread_count += 1;
                }
            }

            write_names(&dir, &names);
        }
    }

    println!("Final thread-count: {} threads.", thread_count);
}

fn main() {
    println!("This is CHARON, the Boards-backupper.");

    let mut processed_ids: HashSet<(String, String)> = HashSet::new();
    let mut nums = std::collections::HashMap::new();


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

    let dirname = unsafe { format!("./backup_{}_{}", REGION, LANGUAGE) };
    let dir = std::path::Path::new(&dirname);
    let _ = std::fs::create_dir(dir.clone());

    let mut names: HashMap<String, bool> = load_names(&dir);

    println!("Loaded {} names.", names.len());

    println!("Do you want to download the Red Tracker or do a forum crawl? [red/crawl]");
    let mut line = {
        let mut line = String::new();
        std::io::stdin().read_line(&mut line).unwrap();
        line.trim_end().to_string()
    };

    while line != "red" && line != "crawl" {
        println!("You must enter either 'red' or 'crawl'.");
        line = {
            let mut line = String::new();
            std::io::stdin().read_line(&mut line).unwrap();
            line.trim_end().to_string()
        };
    }

    if line == "red" {
        handle_reds(&dir, &mut nums, &mut names, &mut processed_ids);
    } else {
        if names.len() == 0 {
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

            unsafe {
                println!("You are now downloading {}'s posts from the {} region and {} language into {}.", line, REGION, LANGUAGE, dir.display());
            }
        }

        handle_names(&dir, &mut nums, &mut names, &mut processed_ids);
    }
}
