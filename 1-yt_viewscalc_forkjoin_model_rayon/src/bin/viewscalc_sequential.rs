use std::{
    collections::HashMap,
    fs::read_dir,
    io::{BufRead, BufReader},
};

fn run_sequential_viewscount() -> Result<(), Box<dyn std::error::Error>> {
    let result = read_dir("./data")?
        .map(|direntry| direntry.unwrap().path())
        .flat_map(|path| {
            let file = std::fs::File::open(path).unwrap();
            let reader = BufReader::new(file);
            reader.lines().skip(1)
        })
        .map(|l| {
            let line = match l {
                Ok(line) => line,
                Err(_) => {
                    return (String::from(""), 0);
                }
            };
            let line = line.replace("\"", "");
            let line = line.replace(",,", ",0,");

            let mut split = line.split(",");

            let channel_title = match split.nth(4) {
                Some(title) => String::from(title),
                None => String::from(""),
            };

            let views = match split.nth(3) {
                Some(views) => match views.parse::<u64>() {
                    Ok(views) => views,
                    Err(_) => 0,
                },
                None => 0,
            };

            (channel_title, views)
        })
        .fold(HashMap::new(), |mut acc, (channel_title, views)| {
            let entry = acc.entry(channel_title).or_insert(0);
            *entry += views;
            acc
        });

    let mut result_vec: Vec<(String, u64)> = result.into_iter().collect();
    result_vec.sort_by(|a, b| b.1.cmp(&a.1));

    let top10_by_total_views = result_vec.into_iter().take(10).collect::<Vec<_>>();

    println!("  Top 10 channels:");
    for (channel_title, views) in top10_by_total_views {
        println!(
            "\t{}: {}",
            channel_title,
            views
                .to_string()
                .chars()
                .rev()
                .collect::<Vec<_>>()
                .chunks(3)
                .map(|chunk| chunk.iter().collect::<String>())
                .collect::<Vec<_>>()
                .join(",")
                .chars()
                .rev()
                .collect::<String>()
        );
    }
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n  Starting benchmark for secuential viewcount calculation per channel in the datasets...\n");

    let start_time = std::time::Instant::now();

    let result = run_sequential_viewscount();
    match result {
        Ok(_) => println!("\n  Benchmark finished successfully!\n"),
        Err(err) => {
            println!("\n  Benchmark failed!\n");
            return Err(err);
        }
    }

    println!("  Total time: {:?}\n", start_time.elapsed());

    Ok(())
}
