extern crate walkdir;
use sha256::digest_file;
use std::collections::HashMap;
use std::fs;
use std::io::stdin;
use std::path::{Path, PathBuf};
use std::process;
use std::sync::mpsc::channel;
use std::sync::{Arc, Mutex};
use threadpool::ThreadPool;
use walkdir::WalkDir;

struct FileEntry {
    path: PathBuf,
    sha256: String,
}

fn main() {
    let mut file_infos: HashMap<String, FileEntry> = HashMap::new();
    let mut files_to_remove: Vec<FileEntry> = Vec::new();

    println!("This is a W.I.P. program, use on your own risk. Description will follow");
    println!("Enter the root path:");
    let mut input_path_string = String::new();
    stdin()
        .read_line(&mut input_path_string)
        .ok()
        .expect("Failed to read line");

    //input_path_string = "Y:\\something\\dontKnow\\11Backup von LF G\n".to_string();
    if input_path_string.ends_with('\n') {
        input_path_string.pop();
        if input_path_string.ends_with('\r') {
            input_path_string.pop();
        }
    }
    input_path_string = input_path_string.replace(r"\", r"/");

    //let path = Path::new(&input_path_string);

    println!("Using {} for recursive duplicate search", input_path_string); // Todo expect
    let path = input_path_string.clone();

    parse_data(
        path, // Todo expect
        &mut file_infos,
        &mut files_to_remove,
    );

    println!(
        "Found {} duplicate files. Show files (y/n):",
        files_to_remove.len()
    );
    let mut show_files_string = String::new();

    stdin()
        .read_line(&mut show_files_string)
        .ok()
        .expect("Failed to read line");

    if show_files_string.ends_with('\n') {
        show_files_string.pop();
        if show_files_string.ends_with('\r') {
            show_files_string.pop();
        }
    }
    if show_files_string.eq("y") {
        for a in files_to_remove.iter() {
            println!("{} {}", a.path.display(), a.sha256);
        }
    }
    println!("What should I do with the files: Remove / Move / Quit  (R/M/Q):",);
    let mut final_job_string = String::new();
    stdin()
        .read_line(&mut final_job_string)
        .ok()
        .expect("Failed to read line");
    if final_job_string.ends_with('\n') {
        final_job_string.pop();
        if final_job_string.ends_with('\r') {
            final_job_string.pop();
        }
    }
    if final_job_string.eq("Q") {
        println!("Goodbye");
        process::exit(1);
    } else if final_job_string.eq("M") {
        move_files(files_to_remove);
    } else if final_job_string.eq("R") {
        remove_files(files_to_remove);
    }
}

fn move_files(f: Vec<FileEntry>) {
    let mut dest_input_string = String::new();
    println!("Please enter the destination directory:");
    stdin()
        .read_line(&mut dest_input_string)
        .ok()
        .expect("Failed to read line");
    if dest_input_string.ends_with('\n') {
        dest_input_string.pop();
        if dest_input_string.ends_with('\r') {
            dest_input_string.pop();
        }
    }
    
    dest_input_string = dest_input_string.replace(r"\", r"/");
    let dest = Path::new(&dest_input_string);
    fs::create_dir_all(dest).expect(" 1234");
    for a in f {
        let p = a.path.clone();
        println!(
            "Moving file: {} to {}",
            p.display(),
            dest.join(a.path.file_name().expect("msg")).display()
        );

        fs::rename(p, dest.join(a.path.file_name().expect("msg"))).expect("BAD msg");
    }
}

fn remove_files(f: Vec<FileEntry>) {
    for a in f {
        println!("Removing file: {:?}", a.path);
        fs::remove_file(a.path).expect("Couldn't remove file");
    }
}

fn parse_data(
    target_path: String,
    file_infos: &mut HashMap<String, FileEntry>,
    files_to_remove: &mut Vec<FileEntry>,
) {
    let progress_counter: Arc<Mutex<i32>> = Arc::new(Mutex::new(0));
    let read_size_sum: Arc<Mutex<u64>> = Arc::new(Mutex::new(0));
    let pool = ThreadPool::new(num_cpus::get()); // fetch the num of cpus to handle madx threads
    let (tx, rx) = channel();

    println!(
        "Starting search, working on max {} threads with root: {}",
        pool.max_count(),
        &target_path
    );

    for file in WalkDir::new(&target_path)
        .into_iter()
        .filter_map(|file| file.ok())
        .filter(|f| f.metadata().unwrap().is_file())
    {
        let tx = tx.clone();

        let progress_counter_copy = Arc::clone(&progress_counter);
        let read_size_sum_copy = Arc::clone(&read_size_sum);
        let pool_copy = pool.clone();
        pool.execute(move || {
            if let Ok(metadata) = file.metadata() {
                // Now let's show our entry's permissions!
                *read_size_sum_copy.lock().unwrap() += metadata.len();
            } else {
                println!("Couldn't get metadata for {:?}", file.path());
            }
            *progress_counter_copy.lock().unwrap() += 1;
            if *progress_counter_copy.lock().unwrap() % 1 == 0 {
                print!(
                    "\rProgress: {} files read. Read: {} Mbytes. Current threads: {}, current queue: {}",
                    *progress_counter_copy.lock().unwrap(),
                    *read_size_sum_copy.lock().unwrap() / 1000000,
                    pool_copy.active_count(),
                    pool_copy.queued_count()
                );
            }
            let h = sha256_digest(&file.path());
            tx.send((h, file)).expect("oops! something went horribly wrong!");
        });
    }
    /*
    println!(
        "Threads started, working on: {} threads with a queue of {}",
        pool.active_count(),
        pool.queued_count()
    );
    */
    pool.join();
    println!("");
    println!("All threads done");
    drop(tx);
    let mut simple_counter: i64 = 0;
    for (h, file) in rx.iter() {
        simple_counter += 1;
        print!(
            "\rProgress {:.3}%",
            (simple_counter as f64 / *progress_counter.lock().unwrap() as f64) * 100 as f64
        );
        if file_infos.contains_key(&h) {
            let foo = FileEntry {
                path: file.path().to_path_buf(),
                sha256: h,
            };
            //println!("{} {}", "Found one", foo.path.display());
            files_to_remove.push(foo);
        } else {
            let k = h.to_owned();
            let foo = FileEntry {
                path: file.path().to_path_buf(),
                sha256: h,
            };
            //println!("{} {} {}", "111111111111Found one", foo.path.display(), k);
            file_infos.insert(k, foo);
        }
    }
    println!("");
}

fn sha256_digest(input: &Path) -> String {
    //let input = Path::new(&file_string);
    let val = digest_file(input).unwrap();

    return val;
}
