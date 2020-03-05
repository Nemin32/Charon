#[macro_use]
extern crate lazy_static;

pub mod common;
pub mod thread;

use crate::common::*;
use crate::thread::*;

lazy_static! {
    static ref CPU_COUNT: usize = num_cpus::get();
}

fn download_frontpage(region: String, page: usize) -> serde_json::Value {
    let request = make_request(format!("/api/{}/discussions?page_size=1000&num_loaded={}", region, page*1000));
    serde_json::from_str(&request).unwrap()
}

fn get_ids(frontpage: serde_json::Value) -> Vec<(String, String)> {
    use regex::Regex;
    lazy_static! {
        static ref REG: Regex = Regex::new(r#"data-application-id="(.*?)" data-discussion-id="(.*?)""#).unwrap();
    }

    let results: Vec<(String, String)> = vec![];
    for capture in REG.captures_iter(frontpage["discussions"].as_str().unwrap()) {
        if let Some(app) = capture.get(1) {
            if let Some(disc) = capture.get(2) {
                results.push((String::from(app.as_str()), String::from(disc.as_str())));
            }
        }
    }

    results
}

fn download_thread(app: String, disc: String) -> Thread {
    let request = make_request(format!("/api/{}/discussions/{}?page_size=1000", app, disc));
    let json = serde_json::from_str(&request).unwrap();

    crate::common::process_raw_thread(json, true)
}

fn catch_up(threads: &mut Vec<std::thread::JoinHandle<Thread>>, posts: &mut Vec<Thread>) {
    for thread in threads.drain(..) {
        if let Ok(post) = thread.join() {
            posts.push(post)
        }
    }
}

fn main() {
    use std::convert::TryInto;

    let region = String::from("q98U6Ykw");
    let mut post_number = 0;
    let mut page = 0;

    loop {
        let frontpage_json: serde_json::Value = download_frontpage(region, page);
        let max_posts = frontpage_json["totalDiscussionsSoft"].as_u64().unwrap().try_into().unwrap();
        let ids: Vec<(String, String)> = get_ids(frontpage_json);

        let mut threads = vec![];
        let mut posts = vec![];

        for (app, disc) in &ids {
            println!("Doing {}/{}", app, disc);

            let app = app.clone();
            let disc = disc.clone();
            threads.push(std::thread::spawn(move || download_thread(app, disc)));
            if threads.len() >= 12 {catch_up(&mut threads, &mut posts)};
        }

        catch_up(&mut threads, &mut posts);

        write_posts(&mut posts, &mut post_number);

        if page*1000 > max_posts {break;}
        page += 1;
    }
}
