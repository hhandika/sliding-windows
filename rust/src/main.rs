use clap::Parser;
use std::collections::BTreeMap;
use std::io::{BufReader, BufWriter};
use std::{fs::File, io::prelude::*, path::Path, path::PathBuf};

const WINDOW_SIZE: &str = "2000000";

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    input: PathBuf,
    #[arg(short, long)]
    output: PathBuf,
    #[arg(short, long, default_value = WINDOW_SIZE)]
    window_size: usize,
}

fn main() {
    let command = Args::parse();
    let data = parse_file(&command.input);
    let average = compute_average(&data, command.window_size);
    let mut output = command.output;
    output.set_extension("csv");
    write_output(&average, &output);
}

fn compute_average(
    data: &BTreeMap<String, Vec<SlidingWindows>>,
    window_size: usize,
) -> BTreeMap<String, Vec<AverageSlidingWindows>> {
    let mut average: BTreeMap<String, Vec<AverageSlidingWindows>> = BTreeMap::new();

    data.iter().for_each(|(key, value)| {
        let mut lowest_start: usize = value[0].start;
        let mut highest_end: usize = value[0].end;
        let mut total_recomb: f64 = 0.0;
        let mut interval_window = 0;
        let mut chrom = String::new();

        value.iter().for_each(|v| {
            // We insert when it reaches the window size or when the chromosome changes
            if interval_window >= window_size || &chrom != key && !chrom.is_empty() {
                let mean_recomb = total_recomb / interval_window as f64;
                average
                    .entry(chrom.clone())
                    .or_insert_with(Vec::new)
                    .push(AverageSlidingWindows {
                        start: lowest_start,
                        end: highest_end,
                        mean_recomb: mean_recomb,
                    });
                lowest_start = v.start;
                highest_end = v.end;
                total_recomb = 0.0;
            }
            if v.start < lowest_start {
                lowest_start = v.start;
            }
            if v.end > highest_end {
                highest_end = v.end;
            }
            chrom = key.to_string();
            let interval = v.end - v.start;
            let recomb_rate = v.recomb_rate * interval as f64;
            total_recomb += recomb_rate;
            interval_window = highest_end - lowest_start;
        });

        // Insert last remaining values for each chromosome
        if total_recomb != 0.0 {
            let mean_recomb = total_recomb / interval_window as f64;
            average
                .entry(key.to_string())
                .or_insert_with(Vec::new)
                .push(AverageSlidingWindows {
                    start: lowest_start,
                    end: highest_end,
                    mean_recomb: mean_recomb,
                });
        }
    });

    average
}

fn write_output(data: &BTreeMap<String, Vec<AverageSlidingWindows>>, output: &Path) {
    let output = File::create(output).unwrap();

    let mut writer = BufWriter::new(output);

    writeln!(writer, "chromosome,interval,mean_recomb_rate").unwrap();

    data.iter().for_each(|(key, value)| {
        value.iter().for_each(|v| {
            writeln!(writer, "{},{}-{},{}", key, v.start, v.end, v.mean_recomb).unwrap();
        });
    });
}

fn parse_file(path: &Path) -> BTreeMap<String, Vec<SlidingWindows>> {
    let file = File::open(path).unwrap();
    let buff = BufReader::new(file);

    let mut data: BTreeMap<String, Vec<SlidingWindows>> = BTreeMap::new();

    buff.lines().map_while(Result::ok).skip(1).for_each(|line| {
        let line: Vec<&str> = line.split_whitespace().collect();

        if line.len() != 5 {
            panic!("Invalid line: {}", line.len());
        }

        let values = SlidingWindows {
            start: line[1].parse().unwrap_or(0),
            end: line[2].parse().unwrap_or(0),
            recomb_rate: line[4].parse().unwrap_or(0.0),
        };

        if values.end != 0 {
            data.entry(line[0].to_string())
                .or_insert_with(Vec::new)
                .push(values);
        }
    });

    data
}

#[derive(Debug)]
struct SlidingWindows {
    start: usize,
    end: usize,
    recomb_rate: f64,
}

#[derive(Debug)]
struct AverageSlidingWindows {
    start: usize,
    end: usize,
    mean_recomb: f64,
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_parse_file() {
        let data = parse_file(Path::new("tests/test.txt"));
        assert_eq!(data.len(), 2);
        assert_eq!(data.get("chr1").unwrap().len(), 2);
        assert_eq!(data.get("chr1").unwrap()[0].start, 0);
        assert_eq!(data.get("chr1").unwrap()[0].end, 200);
        assert_eq!(data.get("chr1").unwrap()[0].recomb_rate, 0.1);
        assert_eq!(data.get("chr1").unwrap()[1].start, 200);
        assert_eq!(data.get("chr1").unwrap()[1].end, 300);
        assert_eq!(data.get("chr1").unwrap()[1].recomb_rate, 0.4);
    }

    #[test]
    fn test_compute_average() {
        let data = parse_file(Path::new("tests/test.txt"));
        let average = compute_average(&data, 300);
        average.iter().for_each(|(key, value)| {
            println!("{}: {:?}", key, value);
        });
        assert_eq!(average.len(), 2);
        assert_eq!(average.get("chr1").unwrap()[0].start, 0);
        assert_eq!(average.get("chr1").unwrap()[0].end, 300);
        assert_eq!(average.get("chr1").unwrap()[0].mean_recomb, 0.2);
    }
}
