use clap::Subcommand;
use simsort::minhash::*;
use simsort::tsp::*;
use simsort::*;

use std::fs::File;
use std::io::Error;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

use clap::Parser;

// Tools to help understand simsort behavior 

fn main() {
    let parse = AnalyzeArgs::try_parse();
    match parse {
        Ok(args) => match args.command {
            Command::PathDistance { filelist } => {
                println!(
                    "distance is {:?}",
                    calculate_path_distance_from_file(&PathBuf::from(filelist))
                );
            }
            Command::FileDistance {
                filelist,
                targetfile,
            } => {
                match calculate_file_distances(&filelist, &targetfile) {
                    Ok(distance_pairs) => {
                        for pair in distance_pairs {
                            println!("{:?} {:?}", pair.1, pair.0.get_path());
                        }
                    },
                    Err(err) => {
                        eprintln!("Error: {}", err);
                        std::process::exit(1);
                    }
                }
            }
        },
        Err(err) => {
            eprintln!("Error: {}", err);
            std::process::exit(1);
        }
    }
}

fn read_files(filepath: &Path) -> Vec<MinhashTarget> {
    let file = File::open(filepath).unwrap();
    let reader = BufReader::new(file);

    let lines: Vec<String> = reader.lines().collect::<Result<_, _>>().unwrap();
    lines
        .iter()
        .map(|f| {
            // The archives I'm generating in my tests are relative to the fs root
            let mut adjusted = "/".to_string();
            adjusted.push_str(f);
            let path = PathBuf::from(adjusted);
            if path.is_dir() {
                MinhashTarget::Directory(path)
            } else {
                MinhashTarget::File(path)
            }
        })
        .collect()
}

//  Takes the output of tar --list, and calculates the path distance for that archive
fn calculate_path_distance_from_file(filepath: &Path) -> u64 {
    let targets = read_files(filepath);
    let (distances, hashed_files, _) = compute_distances(targets);
    let tsp = Tsp::new(distances, hashed_files.len());
    let indices: Vec<usize> = (0..hashed_files.len()).collect();
    tsp.calculate_distance(&indices)
}

// Given the output of tar --list in a file, calculates the similarity score from targetfile to all other files in the list 
fn calculate_file_distances(filepath: &Path, targetfile: &Path) -> Result<Vec<(MinhashTarget, f64)>, Error> {
    let mut distances = Vec::new();
    let file_minhash = minhash_stream(&MinhashTarget::File(targetfile.to_path_buf()))?;
    let targets: Vec<MinhashTarget> = read_files(filepath);
    for target in targets {
        let minhash = minhash_stream(&target)?;
        let distance = file_minhash.score(&minhash);
        distances.push((target.clone(), distance));
    }
    Ok(distances)
}

#[derive(Parser, Debug)]
pub struct AnalyzeArgs {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Clone, Subcommand)]
enum Command {
    PathDistance {
        filelist: PathBuf,
    },
    FileDistance {
        filelist: PathBuf,
        targetfile: PathBuf,
    },
}
