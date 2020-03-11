#[macro_use]
extern crate lazy_static;

pub mod common;
pub mod thread;

use crate::common::*;
use crate::thread::*;

lazy_static! {
    static ref CPU_COUNT: usize = num_cpus::get();
}

fn download_frontpage(region: &String, page: usize) -> serde_json::Value {
    let request = make_request(format!("/api/{}/discussions?sort_type=recent&page_size=1000&num_loaded={}", region, page*1000));
    serde_json::from_str(&request).unwrap()
}

fn get_ids(frontpage: serde_json::Value) -> Vec<(String, String)> {
    use regex::Regex;
    lazy_static! {
        static ref REG: Regex = Regex::new(r#"data-application-id="(.*?)" data-discussion-id="(.*?)""#).unwrap();
    }

    let mut results: Vec<(String, String)> = vec![];
    for capture in REG.captures_iter(frontpage["discussions"].as_str().unwrap()) {
        if let Some(app) = capture.get(1) {
            if let Some(disc) = capture.get(2) {
                results.push((String::from(app.as_str()), String::from(disc.as_str())));
            }
        }
    }

    results
}

fn download_thread(app: String, disc: String) -> Option<Thread> {
    let request = make_request(format!("/api/{}/discussions/{}?page_size=1000", app, disc));

    if let Ok(json)= serde_json::from_str(&request) {
        let json: serde_json::Value = json;
        return Some(crate::common::process_raw_thread(&json["discussion"], true, disc));
    }

    None
}

fn catch_up(threads: &mut Vec<std::thread::JoinHandle<Option<Thread>>>, posts: &mut Vec<Thread>) {
    //println!("Catching up...");
    for thread in threads.drain(..) {
        if let Ok(option) = thread.join() {
            if let Some(post) = option {
                posts.push(post)
            } else {
                println!("Couldn't process thread.");
            }
        }
    }
}

fn write_posts(region: &String, posts: &Vec<Thread>, post_number: &mut usize) {
    let path_string = format!("./backup_{}", region);
    let path = std::path::Path::new(&path_string);

    for post in posts {
        let filepath = path.join(format!("{}.json", post_number));
        let mut file = std::fs::File::create(filepath).unwrap();
        serde_json::to_writer(&mut file, &post).unwrap();

        *post_number+=1;
    }
}

fn get_previous(path: &std::path::Path) -> usize {
    let folder = std::fs::read_dir(path).unwrap().count();

    println!("Found {} files.", folder);

    folder
}

fn main() {
    use std::convert::TryInto;


    let mut region = String::new();

    while region == "" || region == "help" || region.len() != 8 {
        println!("Please enter a region ID (enter 'help' if you don't know what this is):");
        region = {
            let mut region = String::new();
            std::io::stdin().read_line(&mut region).unwrap();
            region.trim_end().to_string()
        };

        if region == "help" {
            println!("\nA region ID is the ID of the forum itself. The easiest way to get this is to collect it using a browser's inspect element function.\nSimply right click on the button that loads more threads on the front page and select Inspect Element (or however it is named on your browser.)\nAfterwards look for a line like this:\n\n'<a data-href=\"/api/[A sequence of characters and numbers]/discussions?sort_type=hot\" class=\"box show-more\" data-count=\"50\">'\n\nWhat you need is the sequence between brackets. For example:\n\n'<a data-href=\"/api/q98U6Ykw/discussions?sort_type=hot\" class=\"box show-more\" data-count=\"50\">'\n\nThe region ID is: q98U6Ykw\n\n")
        } else {
            if region.len() != 8 {
                println!("Region IDs are 8 characters in length.");
            }
        }
    }

    let path_string = format!("./backup_{}", region);
    let path = std::path::Path::new(&path_string);
    let _ = std::fs::create_dir(&path);

    let previous = get_previous(&path);
    let mut page = previous / 1000;
    let mut post_number = previous;
    let mut downloaded = previous;

    unsafe {BASEURL = String::from("boards.eune.leagueoflegends.com");};

    loop {
        let frontpage_json: serde_json::Value = download_frontpage(&region, page);
        let max_posts = frontpage_json["totalDiscussionsSoft"].as_u64().unwrap().try_into().unwrap();
        let ids: Vec<(String, String)> = get_ids(frontpage_json);

        let mut posts = vec![];
        let mut threads = vec![];

        println!("Page {}/{}", page, max_posts/1000);
        for (app, disc) in &ids {
            let app = app.clone();
            let disc = disc.clone();
            threads.push(std::thread::spawn(move || download_thread(app, disc)));
            if threads.len() >= *CPU_COUNT {catch_up(&mut threads, &mut posts)};

            /*downloaded += 1;
            if downloaded % 100 == 0 {
                println!("{}/{}", downloaded, max_posts);
            }*/
        }

        catch_up(&mut threads, &mut posts);
        println!("Writing posts.");
        write_posts(&region, &posts, &mut post_number);

        if page*1000 > max_posts {break;}
        page += 1;
    }

    println!("Final thread count: {}", post_number);
}
