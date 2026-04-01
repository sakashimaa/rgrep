use clap::Parser;
use rayon::prelude::*;
use regex::Regex;
use std::fs;
use std::path::{Path, PathBuf};
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

    #[arg(short, long)]
    count: bool,

    #[arg(short = 'l', long = "files-with-matches")]
    files_with_matches: bool,

    #[arg(short = 'C', long)]
    context: Option<usize>,
}

struct SearchOptions {
    count: bool,
    files_with_matches: bool,
    context: Option<usize>,
}

fn main() {
    let args = Args::parse();

    let pattern = if args.ignore_case {
        format!("(?i){}", args.pattern)
    } else {
        args.pattern.to_string()
    };

    let sopts = &SearchOptions {
        count: args.count,
        files_with_matches: args.files_with_matches,
        context: args.context,
    };

    let regex = match Regex::new(&pattern) {
        Ok(e) => e,
        Err(err) => {
            eprintln!("Invalid regex passed: {}", err);
            return;
        }
    };

    if args.recursive {
        let paths: Vec<PathBuf> = WalkDir::new(&args.filepath)
            .into_iter()
            .filter_map(|entry| match entry {
                Ok(e) if e.file_type().is_file() => Some(e.into_path()),
                Ok(_) => None,
                Err(err) => {
                    eprintln!("Error: {}", err);
                    None
                }
            })
            .collect();
        paths
            .par_iter()
            .for_each(|path| search_file(path, &regex, sopts))
    } else {
        search_file(Path::new(&args.filepath), &regex, sopts);
    }
}

fn search_file(path: &Path, regex: &Regex, opts: &SearchOptions) {
    let contents = match fs::read_to_string(path) {
        Ok(e) => e,
        Err(err) => {
            eprintln!("Error reading: {}", err);
            return;
        }
    };

    if opts.files_with_matches {
        if contents.lines().any(|line| regex.is_match(line)) {
            println!("{}", path.display());
        }

        return;
    }

    if opts.count {
        let count = contents.lines().filter(|line| regex.is_match(line)).count();
        println!("{}:{}", path.display(), count);
        return;
    }

    let matches = contents.lines().enumerate().filter_map(|(num, line)| {
        regex.find(line).map(|m| {
            let colored = format!(
                "{}\x1b[31m{}\x1b[0m{}",
                &line[..m.start()],
                &line[m.start()..m.end()],
                &line[m.end()..]
            );
            (num, colored)
        })
    });

    match opts.context {
        Some(ctx) => {
            // Находим все строки в файле и собираем их в вектор
            let lines: Vec<&str> = contents.lines().collect();

            // Идем и собираем индексы метчей
            let match_indices: Vec<usize> = lines
                .iter()
                .enumerate()
                .filter(|(_, line)| regex.is_match(line))
                .map(|(i, _)| i)
                .collect();

            // Если ничего не нашли не идем дальше
            if match_indices.is_empty() {
                return;
            }

            // Переменная - счетчик отслеживающая индекс элемента который мы последний вывели
            let mut last_printed: Option<usize> = None;
            for &match_idx in &match_indices {
                // Имбовая функция - если к примеру будет 2 - 3 то мы уйдем в ноль и правильно
                // распечатаем
                let start = match_idx.saturating_sub(ctx);

                // End index - делаем нормализацию также чтоб не выйти за границы
                let end = (match_idx + ctx).min(lines.len() - 1);

                // Если последний равен последнему выводу значит мы вывели "ДО" метча?
                if let Some(last) = last_printed
                    && start > last + 1
                {
                    println!("--");
                }

                // Итерируемся по границам контекста
                for i in start..=end {
                    // Непонятно
                    if let Some(last) = last_printed {
                        if i <= last {
                            continue;
                        }
                    }

                    // Если это метч то выводим сам метч цветом и контекстом до него и после него
                    // строки
                    if match_indices.contains(&i) {
                        let m = regex.find(lines[i]).unwrap();
                        println!(
                            "{}:{}:{}\x1b[31m{}\x1b[0m{}",
                            path.display(),
                            i + 1,
                            &lines[i][..m.start()],
                            &lines[i][m.start()..m.end()],
                            &lines[i][m.end()..]
                        )
                    } else {
                        println!("{}-{}-{}", path.display(), i + 1, lines[i]);
                    }
                }

                last_printed = Some(end);
            }
        }
        None => {
            matches.for_each(|(num, colored_line)| {
                println!("{}:{}:{}", path.display(), num + 1, colored_line);
            });
        }
    }
}
