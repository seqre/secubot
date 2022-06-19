use lazy_static::lazy_static;
use regex::Regex;
use serenity::utils::MessageBuilder;
use url_encoded_data::UrlEncodedData;

mod trackers;

use self::trackers::TRACKERS;

pub fn clean_urls(content: &str) -> String {
    lazy_static! {
        static ref RE: Regex = Regex::new(r"http[s]?://[\S]*").unwrap();
    }

    let urls: Vec<_> = RE
        .find_iter(content)
        .map(|mat| mat.as_str().to_string())
        .collect();

    let clean_urls = urls
        .iter()
        .map(|u| {
            let mut url = UrlEncodedData::parse_str(u);
            let pairs = &url.as_string_pairs();
            for (k, _) in pairs.iter() {
                if TRACKERS.contains(k as &str) {
                    url.delete(k);
                }
            }
            let mut response = url.to_final_string();
            if url.is_empty() {
                response.pop();
            }
            response
        })
        .collect();

    format_message(clean_urls)
}

fn format_message(urls: Vec<String>) -> String {
    if !urls.is_empty() {
        let mut msg = MessageBuilder::new();
        msg.push_bold_line("Sanitized URLs");

        for url in urls {
            msg.push_line(format!(" - {}", url));
        }

        return msg.build();
    }
    String::from("")
}
