use clap::Parser;
use regex::Regex;
use std::fs;
use std::path::Path;
use walkdir::WalkDir;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    pattern: String,
    filepath: String,

    #[arg(short, long)]
    ignore_case: bool,

    #[arg(short, long)]
    recursive: bool,
}

fn main() {
    let args = Args::parse();

    let pattern = if args.ignore_case {
        format!("(?i){}", args.pattern)
    } else {
        args.pattern.to_string()
    };

    let regex = match Regex::new(&pattern) {
        Ok(e) => e,
        Err(err) => {
            eprintln!("Invalid regex passed: {}", err);
            return;
        }
    };

    if args.recursive {
        for entry in WalkDir::new(args.filepath) {
            let entry = match entry {
                Ok(e) => e,
                Err(err) => {
                    eprintln!("Error: {}", err);
                    continue;
                }
            };
            if entry.file_type().is_file() {
                let path = entry.path();

                search_file(path, &regex);
            }
        }
    } else {
        search_file(Path::new(&args.filepath), &regex);
    }
}

fn search_file(path: &Path, regex: &Regex) {
    let contents = match fs::read_to_string(path) {
        Ok(e) => e,
        Err(err) => {
            eprintln!("Error reading: {}", err);
            return;
        }
    };

    contents
        .lines()
        .enumerate()
        .filter_map(|(num, line)| {
            regex.find(line).map(|m| {
                let colored = format!(
                    "{}\x1b[31m{}\x1b[0m{}",
                    &line[..m.start()],
                    &line[m.start()..m.end()],
                    &line[m.end()..]
                );
                (num, colored)
            })
        })
        .for_each(|(num, colored_line)| {
            println!("{}:{}:{}", path.display(), num + 1, colored_line);
        });
}
