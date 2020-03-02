use std::collections::{HashSet, HashMap};
use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};

use crate::common::*;
use crate::thread::*;

fn get_rioter_profiles(json: serde_json::Value) -> HashSet<String> {
    use regex::Regex;
    lazy_static! {
        //static ref REDREG: Regex = Regex::new(r#"href=.(.*?)\?"#).unwrap();
        static ref REDREG: Regex = Regex::new(r#"author byline.>\n\t\t\t<a href=.(.*?).\n\t"#).unwrap();
    }

    let mut results: HashSet<String> = HashSet::new();

    if let Some(json_results) = json["items"].as_str() {
        for capture in REDREG.captures_iter(&json_results) {
            results.insert(String::from(&capture[1]));
        }
    }

    results
}

fn get_redtracker_profiles(created_to: &str, hs: &mut HashSet<String>) {
    let request: serde_json::Value = unsafe { serde_json::from_str(&make_request(format!("/{}/redtracker?json_wrap=1&created_to={}", LANGUAGE, created_to))).unwrap() };

    if let Some(next) = request["lastCreated"].as_str() {
        let next = next.clone();
        let next_escaped = utf8_percent_encode(next, NON_ALPHANUMERIC).to_string();
        hs.extend(get_rioter_profiles(request));

        println!("Found {} Rioters.", hs.len());

        if created_to != next_escaped {
            get_redtracker_profiles(&next_escaped, hs);
        }
    }
}

fn get_redtracker_profile_ids(profile: &String) -> HashSet<(String, String)> {
    let mut retval = HashSet::new();
    let request = format!("{}?json_wrap=1", profile);

    retval.extend(crate::get_user_ids(request));

    retval
}

fn process_redtracker_profile(name: &String, name_queue: &mut HashMap<String, bool>, processed_ids: &mut HashSet<(String, String)>) -> Vec<Thread>{
    let mut ids = get_redtracker_profile_ids(name);
    crate::prune_ids(&mut ids, processed_ids);
    let (threads, names) = crate::process_threads(&ids);

    crate::add_names(names, name, name_queue);

    threads
}

pub fn handle_reds(dir: &std::path::Path, nums: &mut HashMap<String, usize>, names: &mut HashMap<String, bool>, processed_ids: &mut HashSet<(String, String)>) {
    let mut red_names = HashSet::new();
    get_redtracker_profiles("", &mut red_names);
    red_names.retain(|elem| !names.get(elem).unwrap_or(&false));

    let mut thread_count = 0;

    for (i, name) in red_names.iter().enumerate() {
        print!("[{}/{}] {} ", i, red_names.len(), name);
        let threads = process_redtracker_profile(name, names, processed_ids);

        for post in threads.iter() {
            crate::write_file(&dir, post, nums);
            thread_count += 1;
        }

        crate::write_names(&dir, &names);
    }

    println!("Final thread-count: {} threads.", thread_count);
}
