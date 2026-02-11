use browsy_core::{fetch, output};
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "browsy", about = "Zero-render browser engine for AI agents")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Fetch a URL and output the Spatial DOM
    Fetch {
        /// The URL to fetch
        url: String,

        /// Output as JSON instead of compact format
        #[arg(long)]
        json: bool,

        /// Viewport size as WxH (default: 1920x1080)
        #[arg(long, default_value = "1920x1080")]
        viewport: String,

        /// Skip fetching external CSS stylesheets
        #[arg(long)]
        no_css: bool,
    },
    /// Parse a local HTML string and output the Spatial DOM
    Parse {
        /// The HTML file to parse (use - for stdin)
        file: String,

        /// Output as JSON instead of compact format
        #[arg(long)]
        json: bool,

        /// Viewport size as WxH (default: 1920x1080)
        #[arg(long, default_value = "1920x1080")]
        viewport: String,
    },
}

fn parse_viewport(s: &str) -> (f32, f32) {
    let parts: Vec<&str> = s.split('x').collect();
    if parts.len() == 2 {
        let w = parts[0].parse().unwrap_or(1920.0);
        let h = parts[1].parse().unwrap_or(1080.0);
        (w, h)
    } else {
        (1920.0, 1080.0)
    }
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Fetch {
            url,
            json,
            viewport,
            no_css,
        } => {
            let (vw, vh) = parse_viewport(&viewport);
            let config = fetch::FetchConfig {
                viewport_width: vw,
                viewport_height: vh,
                fetch_css: !no_css,
                ..Default::default()
            };

            match fetch::fetch(&url, &config) {
                Ok(dom) => print_dom(&dom, json),
                Err(e) => {
                    eprintln!("Error: {}", e);
                    std::process::exit(1);
                }
            }
        }
        Commands::Parse {
            file,
            json,
            viewport,
        } => {
            let (vw, vh) = parse_viewport(&viewport);
            let html = if file == "-" {
                use std::io::Read;
                let mut buf = String::new();
                std::io::stdin().read_to_string(&mut buf).expect("Failed to read stdin");
                buf
            } else {
                std::fs::read_to_string(&file).expect("Failed to read file")
            };

            let dom = browsy_core::parse(&html, vw, vh);
            print_dom(&dom, json);
        }
    }
}

fn print_dom(dom: &output::SpatialDom, as_json: bool) {
    if as_json {
        println!("{}", serde_json::to_string_pretty(dom).unwrap());
    } else {
        if !dom.title.is_empty() {
            println!("title: {}", dom.title);
        }
        if !dom.url.is_empty() {
            println!("url: {}", dom.url);
        }
        println!("vp: {}x{}", dom.vp[0] as i32, dom.vp[1] as i32);
        println!("els: {}", dom.els.len());
        println!("---");
        println!("{}", output::to_compact_string(dom));
    }
}
