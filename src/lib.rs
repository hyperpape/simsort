pub mod binsort;
pub mod minhash;
pub mod tour;
pub mod tsp;
pub mod twoopt;
pub mod utils;
#[cfg(test)]
mod testutils;

use crate::binsort::*;
use crate::minhash::*;
use crate::tour::Tour;
use crate::tsp::Tsp;
use crate::twoopt::{optimize_twoopt_from_tour, MINIMUM_ITEMS};

use pathdiff::diff_paths;
use std::collections::HashMap;
use std::env::current_dir;
use std::ffi::OsString;
use std::path::{Component, Path, PathBuf};
use walkdir::WalkDir;

use clap::{Parser, ValueEnum};

pub fn run(args: Args) -> Result<(), i32> {
    utils::perf_trace("Simsort", "Process", "B", utils::get_micros());
    log::info!("Starting to process path {:?}", args.directory);
    match current_dir() {
        Ok(current_dir) => {
            let ordered_files = load_and_order(args)?;
            display_files(current_dir, ordered_files)?;
        }
        Err(s) => {
            log::error!("{}", s);
            utils::perf_trace("Simsort", "Process", "E", utils::get_micros());
            return Err(exitcode::IOERR);
        }
    }
    Ok(())
}

fn load_and_order(args: Args) -> Result<Vec<PathBuf>, i32> {
    match by_filename(Path::new(&args.directory)) {
        Ok(files) => match process(&args, files) {
            Ok(ordered_files) => Ok(ordered_files),
            Err(s) => {
                log::error!("{}", s);
                // TODO: Generic exitcode, reconsider later
                Err(1)
            }
        },
        Err(s) => {
            log::error!("{}", s);
            Err(exitcode::IOERR)
        }
    }
}

fn display_files(current_dir: PathBuf, ordered_files: Vec<PathBuf>) -> Result<(), i32> {
    for f in ordered_files {
        match output_path(&f, &current_dir) {
            Ok(p) => {
                println!("{}", p.display());
            }
            Err(s) => {
                log::error!("{}", s);
                // TODO: Generic exitcode, reconsider later
                return Err(1);
            }
        }
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum Algorithm {
    Tsp,
    OnlyExtensions,
    ByteDistributions,
    BinsortOriginal,
}

impl Algorithm {
    fn max_batch(&self) -> usize {
        match self {
            Self::Tsp => 50000,
            Self::ByteDistributions => usize::MAX,
            Self::OnlyExtensions => usize::MAX,
            Self::BinsortOriginal => 100000,
        }
    }

    fn order<'a>(&self, files: Vec<MinhashTarget>) -> Result<Vec<PathBuf>, String> {
        match self {
            Self::Tsp => order_tsp(files),
            Self::ByteDistributions => convert_to_pathbufs(files),
            Self::OnlyExtensions => convert_to_pathbufs(files),
            Self::BinsortOriginal => Ok(order_binsort(files)),
        }
    }
}

#[derive(Parser, Debug)]
pub struct Args {
    directory: String,
    #[arg(value_enum)]
    algorithm: Algorithm,
}

fn output_path(file_path: &PathBuf, current_dir: &PathBuf) -> Result<PathBuf, String> {
    if !file_path.is_absolute() {
        // TODO: do we need to strip ../?
        return Ok(file_path.clone());
    }
    match diff_paths(file_path, current_dir) {
        Some(p) => {
            let mut resolved_path = PathBuf::new();
            for elem in p.components() {
                match elem {
                    Component::Normal(n) => {
                        resolved_path.push(n);
                    }
                    Component::CurDir => {
                        resolved_path.push(elem);
                    }
                    _ => {
                        // ignore other types of components.
                    }
                }
            }
            Ok(resolved_path)
        }
        None => Err(format!(
            "Failed to find output path for '{}' and '{}'",
            file_path.display(),
            current_dir.display()
        )),
    }
}

fn process(
    args: &Args,
    files: HashMap<OsString, Vec<MinhashTarget>>,
) -> Result<Vec<PathBuf>, String> {
    let size: usize = files.values().map(|v| v.len()).sum();
    log::info!("Processing {:?} files", size);
    match args.algorithm {
        Algorithm::OnlyExtensions => {
            let ordered: Vec<PathBuf> = files
                .into_values()
                .flatten()
                .map(|t| t.get_path().to_path_buf())
                .collect();
            return Ok(ordered);
        }
        _ => {
            return Ok(order_in_batches(&args.algorithm, files)?);
        }
    }
}

fn order_in_batches(
    algorithm: &Algorithm,
    files: HashMap<OsString, Vec<MinhashTarget>>,
) -> Result<Vec<PathBuf>, String> {
    // TODO: may eventually be worth making max batch based on a command line switch--idea being you can choose efficiency or performance
    let mut uniform_pending = Vec::new();
    let mut ascii_pending = Vec::new();
    let mut ascii_processed = Vec::new();
    let mut remainder_pending = Vec::new();
    let mut remainder_processed = Vec::new();
    let mut unhashed = Vec::new();
    for next_files in files.into_values() {
        for target in next_files {
            match minhash_stream(&target) {
                Ok(minhash) => {
                    match minhash.byte_distribution {
                        // TODO: do proper bin-packing here
                        ByteDistribution::Uniform => {
                            uniform_pending.push(target.get_path().to_path_buf());
                        }
                        ByteDistribution::Ascii(_) => {
                            ascii_pending.push(target);
                            if ascii_pending.len() > algorithm.max_batch() {
                                let mut foo = algorithm.order(ascii_pending)?;
                                ascii_processed.append(&mut foo);
                                ascii_pending = Vec::new();
                            }
                        }
                        ByteDistribution::NonAscii(_) => {
                            remainder_pending.push(target);
                            if remainder_pending.len() > algorithm.max_batch() {
                                let mut foo = algorithm.order(remainder_pending)?;
                                remainder_processed.append(&mut foo);
                                remainder_pending = Vec::new();
                            }
                        }
                    }
                }
                Err(error_string) => {
                    log::error!("Failed to read target={:?}, {}", target.get_path(), error_string);
                    // TODO: reconsider error handling logic
                    // An error in reading the file does not mean we're generating an incorrect set of files
                    // Probably the downstream process will also choke on whatever file it is, but I think we don't care? 
                    unhashed.push(target.get_path().to_path_buf());
                }
            }
        }
    }

    let ascii_count = ascii_processed.len() + ascii_pending.len();
    let remainder_count = remainder_processed.len() + remainder_pending.len();
    log::debug!("{} ascii files, {} non-ascii files, {} uniform files, {} unhashed files", ascii_count, remainder_count, uniform_pending.len(), unhashed.len());
    
    let mut ordered = uniform_pending;
    ordered.append(&mut remainder_processed);
    ordered.append(&mut algorithm.order(remainder_pending)?);
    ordered.append(&mut ascii_processed);
    ordered.append(&mut algorithm.order(ascii_pending)?);
    ordered.append(&mut unhashed);
    Ok(ordered)
}

pub fn compute_distances(targets: Vec<MinhashTarget>) -> (Vec<u8>, Vec<PathBuf>, Vec<PathBuf>) {
    let mut simhashes = vec![];
    let mut hashed_files = vec![];
    let mut unhashed_files = vec![];
    utils::perf_trace("Creating simhashes", "Minhash", "B", utils::get_micros());
    for target in targets {
        let start = utils::get_micros();
        let result = minhash_stream(&target);
        match result {
            Ok(hash) => {
                utils::perf_trace("Minhash", "Minhash", "X", start);
                simhashes.push(hash);
                hashed_files.push(target.get_path().to_path_buf());
            }
            Err(_) => {
                // TODO: handle specific errors instead of a catchall
                unhashed_files.push(target.get_path().to_path_buf());
            }
        };
    }
    utils::perf_trace("Creating simhashes", "Minhash", "E", utils::get_micros());
    log::info!("Created simhashes for {} files", hashed_files.len());
    let file_count = hashed_files.len();
    let mut distances = vec![0; file_count * file_count];
    utils::perf_trace("Creating distances", "Distances", "B", utils::get_micros());
    for i in 0..hashed_files.len() {
        utils::perf_trace("Distances for file", "Distances", "B", utils::get_micros());
        for j in i..hashed_files.len() {
            let similarity = simhashes[i].score(&simhashes[j]);
            let distance = 255 - ((similarity * 255.0).floor() as u8);
            distances[i * file_count + j] = distance;
            distances[j * file_count + i] = distance;
        }
        utils::perf_trace("Distances for file", "Distances", "E", utils::get_micros());
    }
    utils::perf_trace("Creating distances", "Distances", "E", utils::get_micros());
    return (distances, hashed_files, unhashed_files);
}

fn convert_to_pathbufs(files: Vec<MinhashTarget>) -> Result<Vec<PathBuf>, String> {
    Ok(files
        .into_iter()
        .map(|target| target.get_path().clone())
        .collect())
}

fn order_tsp(files: Vec<MinhashTarget>) -> Result<Vec<PathBuf>, String> {
    let mut paths = vec![];

    // TODO: we're recomputing the hashes here, which is a waste
    let (distances, hashed_files, mut unhashed_files) = compute_distances(files);
    if hashed_files.len() < MINIMUM_ITEMS {
        paths = hashed_files;
    } else {
        let tsp = Tsp::new(distances, hashed_files.len());
        let tour = Tour::new((0..hashed_files.len()).collect());
        let indices = optimize_twoopt_from_tour(&tsp, tour)?;
        for i in indices {
            // TODO: this feels like an unnecessary clone
            paths.push(hashed_files.get(i).unwrap().clone());
        }
    }
    paths.append(&mut unhashed_files);
    return Ok(paths);
}

fn order_binsort<'a>(files: Vec<MinhashTarget>) -> Vec<PathBuf> {
    let (distances, hashed_files, mut unhashed_files) = compute_distances(files);
    let tsp = Tsp::new(distances, hashed_files.len());
    let indices = optimize_binsort(&tsp);
    let mut paths = vec![];
    for i in indices {
        // TODO: this feels like an unnecessary clone
        paths.push(hashed_files.get(i).unwrap().clone());
    }
    paths.append(&mut unhashed_files);
    return paths;
}

fn by_filename<'a>(dir: &Path) -> Result<HashMap<OsString, Vec<MinhashTarget>>, String> {
    let mut map = HashMap::new();
    for entry in WalkDir::new(dir) {
        match entry {
            Ok(e) => {
                match e.path().is_dir() {
                    true => {
                        map.entry(OsString::from(""))
                            .or_insert(Vec::new())
                            .push(MinhashTarget::Directory(e.path().to_path_buf()));
                    }
                    false => {
                        let pathbuf = e.path().to_path_buf();
                        let extension = pathbuf.extension().unwrap_or_default().to_owned();
                        map.entry(extension)
                            .or_default()
                            .push(MinhashTarget::File(pathbuf));
                    }
                }
            }
            Err(e) => {
                log::error!("Failure to read file, {}", e);
                // TODO: proper error
                return Err("Failure to read file".to_string());
            }
        }
    }
    Ok(map)
}

#[cfg(test)]
mod tests {

    use crate::testutils::*;
    use crate::*;

    use rand::Rng;
    use std::collections::HashSet;
    use std::fs::{create_dir, File};
    use std::io::Write;
    use std::process::Command;
    use tempfile::{tempdir, TempDir};

    #[test]
    fn generate_nearest_neighbor_tour_generates_a_tour() {
        let distances = build_linear_distances(7);
        let tsp = Tsp::new(distances, 7);
        let tour = tsp.generate_nearest_neighbor_tour(3);
        check_permutation(&tour, 6);
    }

    #[test]
    fn output_path_does_stuff() {
        assert_eq!(
            PathBuf::from(""),
            output_path(&PathBuf::from("/a/b/c"), &PathBuf::from("/a/b/c")).unwrap()
        );
        assert_eq!(
            PathBuf::from("c/"),
            output_path(&PathBuf::from("/a/b/c"), &PathBuf::from("/a/b")).unwrap()
        );
        assert_eq!(
            PathBuf::from("c/d.txt"),
            output_path(&PathBuf::from("/a/b/c/d.txt"), &PathBuf::from("/a/b")).unwrap()
        );
    }

    #[test]
    fn load_and_order_returns_all_files() {
        let temp_dir = tempdir().unwrap();
        for i in 0..10 {
            let mut file_name = "file".to_string();
            file_name.push_str(&(i.to_string()));
            let file_path = temp_dir.path().join(file_name);
            let mut file = File::create(file_path).unwrap();
            writeln!(file, "randomdata").unwrap();
        }
        let path = &temp_dir.into_path();
        let directory = path.to_str().unwrap().to_string();

        let args = Args {
            directory: directory,
            algorithm: Algorithm::Tsp,
        };
        let ordered_files = load_and_order(args).unwrap();
        assert_eq!(11, ordered_files.len());
        for i in 0..10 {
            let mut contained = false;
            for path in &ordered_files {
                let mut candidate = "file".to_string();
                candidate.push_str(&(i.to_string()));
                if *path.file_name().unwrap() == *candidate {
                    contained = true;
                }
            }
            assert!(contained);
        }
    }

    #[test]
    fn load_and_order_optimizes_file_order() {
        let temp_dir = tempdir().unwrap();
        for i in 0..10 {
            let mut rng = rand::thread_rng();
            let file_name = i.to_string();
            let file_path = temp_dir.path().join(file_name);
            let mut file = File::create(file_path).unwrap();
            if i % 2 == 0 {
                let random_bytes: Vec<u8> = (0..1024).map(|_| rng.gen()).collect();
                file.write_all(&random_bytes).unwrap();
            } else {
                for _ in 0..1024 {
                    write!(file, "a").unwrap();
                }
            }
        }

        let path = &temp_dir.into_path();
        let directory = path.to_str().unwrap().to_string();

        let args = Args {
            directory: directory,
            algorithm: Algorithm::Tsp,
        };
        let ordered_files = load_and_order(args).unwrap();

        // 10 files, 1 directory
        assert_eq!(11, ordered_files.len());
        // check that we've grouped evens and odds into groups of 5 consecutive files
        for i in 0..3 {
            let file1: u32 = ordered_files[i]
                .file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .parse()
                .ok()
                .unwrap();
            let file2: u32 = ordered_files[i + 1]
                .file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .parse()
                .ok()
                .unwrap();
            assert_eq!(file1 % 2, file2 % 2);
        }
        for i in 5..8 {
            // We have to account for directories not having a filename here, so we only compare when the Option is populated
            let file1: Option<u32> = ordered_files[i].file_name().unwrap().to_str().unwrap().parse().ok();
            let file2: Option<u32> = ordered_files[i + 1].file_name().unwrap().to_str().unwrap().parse().ok();
            if file1.is_some() && file2.is_some() {
                assert_eq!(file1.unwrap() % 2, file2.unwrap() % 2);
            }
        }
    }

    #[test]
    fn run_can_run() {
        let temp_dir = setup_directory(6);
        let args = Args {
            directory: temp_dir
                .into_path()
                .as_os_str()
                .to_str()
                .unwrap()
                .to_string(),
            algorithm: Algorithm::Tsp,
        };
        let _ = run(args);
    }

    #[test]
    fn by_filename_returns_the_same_files_as_tar() {
        let tempdir = setup_directory(1);
        let file_map = by_filename(tempdir.path()).unwrap();

        let files: HashSet<PathBuf> = file_map
            .into_values()
            .flat_map(|v| {
                v.into_iter().map(|t| {
                    t.get_path()
                        .to_path_buf()
                        .strip_prefix(Path::new("/"))
                        .unwrap()
                        .to_path_buf()
                })
            })
            .collect();
        assert_eq!(5, files.len());

        let tar_files = get_tar_files(&tempdir);
        assert_eq!(files, tar_files);
    }

    fn setup_directory(file_count: usize) -> TempDir {
        let temp_dir = tempdir().unwrap();

        let empty_directory = temp_dir.path().join("empty_dir".to_string());
        create_dir(empty_directory).unwrap();

        let hidden_empty_directory = temp_dir.path().join(".hidden_empty_dir".to_string());
        create_dir(hidden_empty_directory).unwrap();

        let populated_path = temp_dir.path().join("populated_directory".to_string());
        create_dir(populated_path.clone()).unwrap();
        for i in 0..file_count {
            let mut file = File::create(populated_path.clone().join(format!("{}", i))).unwrap();
            file.write_all(b"abc").unwrap();
        }
        temp_dir
    }

    fn get_tar_files(filedir: &TempDir) -> HashSet<PathBuf> {
        let output_dir = tempdir().unwrap();
        let archive_path = output_dir.path().join("archive.tar.gz".to_string());
        let archive_path_os_str = archive_path.as_os_str();
        let _ = Command::new("tar")
            .arg("-cf")
            .arg(archive_path_os_str)
            .arg(filedir.path().as_os_str())
            .output();

        let list = Command::new("tar")
            .arg("--list")
            .arg("--file")
            .arg(archive_path_os_str)
            .output()
            .unwrap()
            .stdout;
        let list_str = String::from_utf8(list).unwrap();
        list_str.lines().map(|l| PathBuf::from(l)).collect()
    }

    // Interactive test function for printing the diffs for a set of files
    fn print_diffs() {
        let mut paths = Vec::new();
        let target = PathBuf::from("testdata".to_owned());
        for entry in WalkDir::new(target) {
            match entry {
                Ok(e) => {
                    match e.path().is_dir() {
                        true => {
                            // do nothing
                        }
                        false => {
                            paths.push(e.path().to_path_buf());
                        }
                    }
                }
                Err(e) => {
                    log::error!("{}", e);
                    panic!("can't read");
                }
            }
        }
        for i in 0..paths.len() {
            let mh = minhash_stream(&MinhashTarget::File(paths[i].clone())).unwrap();
            for j in i..paths.len() {
                if i == j {
                    continue;
                }
                let mh2 = minhash_stream(&MinhashTarget::File(paths[j].clone())).unwrap();
                println!(
                    "path1={:?}, path2={:?}, score={}",
                    paths[i],
                    paths[j],
                    mh.score(&mh2)
                );
            }
        }
    }
}
