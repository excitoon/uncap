use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use std::io::Read;
use uncap::solver::SolverKind;

#[derive(Parser)]
#[command(
    name = "uncap",
    version,
    about = "Solve captchas locally — no paid services"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Solve a text/image captcha via OCR
    Text {
        /// Image file (use - for stdin)
        file: String,
    },

    /// Solve a math-expression captcha (e.g. "3 + 7 = ?")
    Math {
        /// Image file (use - for stdin)
        file: String,
    },

    /// Solve an audio captcha via speech-to-text
    Audio {
        /// Audio file (wav/mp3/ogg)
        file: String,
    },

    /// Solve a slide/puzzle captcha — returns the X offset
    Slide {
        /// Image file containing the puzzle background
        file: String,
    },

    /// Crawl URLs looking for captchas
    Crawl {
        /// Seed URLs to start crawling from
        #[arg(required = true)]
        urls: Vec<String>,

        /// Maximum number of pages to visit
        #[arg(short, long, default_value = "50")]
        max_pages: usize,

        /// Download found captchas to this directory
        #[arg(short, long)]
        output: Option<String>,

        /// Also attempt to solve found captchas
        #[arg(long)]
        solve: bool,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::Text { file } => {
            let bytes = read_input(&file)?;
            let result = uncap::solve(&bytes, SolverKind::Text)?;
            println!("{result}");
        }

        Command::Math { file } => {
            let bytes = read_input(&file)?;
            let result = uncap::solve(&bytes, SolverKind::Math)?;
            println!("{result}");
        }

        Command::Audio { file } => {
            let bytes = read_input(&file)?;
            let result = uncap::solve(&bytes, SolverKind::Audio)?;
            println!("{result}");
        }

        Command::Slide { file } => {
            let bytes = read_input(&file)?;
            let result = uncap::solve(&bytes, SolverKind::Slide)?;
            println!("{result}");
        }

        Command::Crawl {
            urls,
            max_pages,
            output,
            solve,
        } => {
            let seeds: Vec<&str> = urls.iter().map(|s| s.as_str()).collect();
            let found = uncap::crawler::crawl(&seeds, max_pages)?;

            if found.is_empty() {
                eprintln!("no captchas found");
                return Ok(());
            }

            for (i, cap) in found.iter().enumerate() {
                println!("[{i}] {:?} — {}", cap.detection, cap.captcha_url);
                println!("    page: {}", cap.page_url);

                if output.is_some() || solve {
                    match uncap::crawler::download(&cap.captcha_url) {
                        Ok(bytes) => {
                            if let Some(ref dir) = output {
                                std::fs::create_dir_all(dir)?;
                                let ext = if cap.captcha_url.contains(".png") {
                                    "png"
                                } else if cap.captcha_url.contains(".jpg")
                                    || cap.captcha_url.contains(".jpeg")
                                {
                                    "jpg"
                                } else if cap.captcha_url.contains(".gif") {
                                    "gif"
                                } else {
                                    "png"
                                };
                                let path = format!("{dir}/captcha_{i:04}.{ext}");
                                std::fs::write(&path, &bytes)?;
                                println!("    saved: {path}");
                            }

                            if solve {
                                match uncap::solve(&bytes, SolverKind::Text) {
                                    Ok(text) => println!("    solved: {text}"),
                                    Err(e) => println!("    solve failed: {e}"),
                                }
                            }
                        }
                        Err(e) => {
                            eprintln!("    download failed: {e}");
                        }
                    }
                }
                println!();
            }

            println!(
                "found {} captcha(s) across {} page(s)",
                found.len(),
                max_pages
            );
        }
    }

    Ok(())
}

fn read_input(file: &str) -> Result<Vec<u8>> {
    if file == "-" {
        let mut buf = Vec::new();
        std::io::stdin().read_to_end(&mut buf)?;
        Ok(buf)
    } else {
        std::fs::read(file).with_context(|| format!("read {file}"))
    }
}
