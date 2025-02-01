use bytesize::ByteSize;
use clap::Parser;
use clashctl_core::ClashBuilder;
use regex::Regex;
use serde_json::json;
use std::fs::read_to_string;

use std::path::Path;
use std::time::Instant;
use std::{thread::sleep, time::Duration};

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    /// Hiddify data directory path
    #[arg(short, long)]
    directory: String,
    /// Update interval in seconds
    #[arg(short, long)]
    #[arg(default_value = "1")]
    interval: u64,
    /// Download stats color
    #[arg(long)]
    #[arg(default_value = "#B985FA")]
    dl_color: String,
    /// Upload stats color
    #[arg(long)]
    #[arg(default_value = "#FA699B")]
    up_color: String,
    /// Filter profile name based on a regex
    #[arg(short, long)]
    pattern: Option<String>,
}

fn format_byte(value: u64) -> String {
    ByteSize(value)
        .to_string_as(false)
        .to_lowercase()
        .replace(" ", "")
}

fn print(text: String, tooltip: String) {
    let data = json!({
    "alt": "",
    "class": "",
    "percentage": 0,
    "text": text,
    "tooltip": tooltip,
    });

    println!("{data}");
}

fn main() {
    let args = Cli::parse();
    let dl_color = args.dl_color;
    let up_color = args.up_color;
    let mut max_up = 0;
    let mut max_down = 0;
    let mut last = Instant::now();

    loop {
        let config: serde_json::Value = serde_json::from_str(
            read_to_string(Path::new(&args.directory).join("current-config.json"))
                .expect("Failed to read `current-config.json`")
                .as_str(),
        )
        .expect("Failed to parse `current-config.json`");

        let clash_api = config
            .get("experimental")
            .unwrap()
            .get("clash_api")
            .unwrap();

        let url = format!(
            "http://{}",
            clash_api
                .get("external_controller")
                .unwrap()
                .as_str()
                .unwrap()
        );

        let secret = clash_api.get("secret").unwrap().as_str().unwrap();

        let clash = ClashBuilder::new(url)
            .unwrap()
            .secret(Some(secret.to_string()))
            .build();

        if let Ok(traffics) = clash.get_traffic() {
            let proxies = clash.get_proxies().unwrap();
            let current_group_name = proxies
                .groups()
                .find(|(name, _)| *name == "GLOBAL")
                .unwrap()
                .1
                .now
                .as_ref()
                .unwrap();

            let mut profile_name = proxies
                .groups()
                .find(|(name, _)| *name == current_group_name)
                .unwrap()
                .1
                .now
                .clone()
                .unwrap();

            if let Some(pattern) = &args.pattern {
                let re = Regex::new(&pattern).expect("Invalid regex pattern");

                if let Some(captures) = re.captures(profile_name.as_str()) {
                    profile_name = String::from(captures.get(0).unwrap().as_str())
                }
            }

            for traffic in traffics {
                match traffic {
                    Ok(traffic) => {
                        let now: Instant = Instant::now();

                        if traffic.up > max_up {
                            max_up = traffic.up
                        }

                        if traffic.down > max_down {
                            max_down = traffic.down
                        }

                        if now.duration_since(last).as_secs() < args.interval {
                            continue;
                        } else {
                            last = now
                        }

                        let current_upload = format_byte(traffic.up) + "/s";
                        let current_download = format_byte(traffic.down) + "/s";
                        let max_upload = format_byte(max_up) + "/s";
                        let max_download = format_byte(max_down) + "/s";
                        let mut total_upload = String::new();
                        let mut total_download = String::new();
                        let mut connection_count = String::new();

                        if let Ok(conn) = clash.get_connections() {
                            connection_count = conn.connections.len().to_string();
                            total_upload = format_byte(conn.upload_total);
                            total_download = format_byte(conn.download_total);
                        }

                        let speed_text = format!("<span foreground='{dl_color}'> {current_download}</span> <span foreground='{up_color}'> {current_upload}</span>");
                        let max_speed_text = format!("Max: <span foreground='{dl_color}'> {max_download}</span> <span foreground='{up_color}'> {max_upload}</span>");
                        let total_text = format!("Total: <span foreground='{dl_color}'> {total_download}</span> <span foreground='{up_color}'> {total_upload}</span>");

                        print(
                            format!("{profile_name}"),
                            format!(
                                " {connection_count} {speed_text}\n{max_speed_text}\n{total_text}"
                            ),
                        )
                    }

                    Err(_) => break,
                }
            }

            print(String::new(), String::new())
        }

        sleep(Duration::from_secs(5))
    }
}
