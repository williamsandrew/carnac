use std::env;
use std::io;
use std::fs;
use std::error::Error;
use std::path::{PathBuf};
// use std::ffi::OsString;
use std::io::prelude::*;
use std::process::{Command, Stdio};

extern crate snap;
extern crate crypto;
use crypto::sha2;
use crypto::digest::Digest;

// const EMPTY_HASH: &'static str =
//     "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";


fn cache_path(hash: &str, compressed: bool) -> PathBuf {
    let mut buf = match env::home_dir() {
        Some(path) => { PathBuf::from(path) },
        None => { panic!("Cant find home directory") },
    };

    buf.push(".carnac");
    buf.push("cache");

    fs::create_dir_all(&buf).unwrap();

    buf.push(hash);
    if compressed {
        buf.set_extension("sz");
    }

    buf
}

fn hash(buf: &[u8]) -> Option<String> {
    if buf.is_empty() {
        return None
    }

    let mut hasher = sha2::Sha256::new();
    hasher.input(&buf);

    let result = hasher.result_str();
    Some(result)
}

fn hash_and_save(buf: &[u8], compress: bool) {
    let hash = match hash(&buf) {
        Some(h) => h,
        None => panic!("got none from hash; expected some"),
    };

    let filename = cache_path(&hash, compress);

    if !filename.exists() {
        let mut file = fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(filename)
            .unwrap();

        let mut cursor = io::Cursor::new(&buf);
        if compress {
            let mut wtr = snap::Writer::new(file);
            io::copy(&mut cursor, &mut wtr).unwrap();
        } else {
            io::copy(&mut cursor, &mut file).unwrap();
        }
    }
}

fn real_main() {
    if env::var("CARNAC_FORK").is_ok() {
        panic!("we appear to be in a loop");
    }

    // let cmd = match env::args().nth(1) {
    //     None => { panic!("please specify cmd") },
    //     Some(c) => c,
    // };

    let cmd = match env::args().nth(1) {
        None => { panic!("please specify cmd") },
        Some(c) => c,
    };

    // TODO Check for --. This isnt right
    let pos = match env::args().nth(2) {
        Some(e) => {
            if e == "--" {
                3
            } else {
                2
            }
        },
        None => { 2 },
    };

    let extra_args = env::args().skip(pos).collect::<Vec<String>>();
    println!("Cmd: {:?}", cmd);
    println!("{:?}", extra_args);


    let process = match Command::new(cmd)
            .args(&extra_args)
            .env("CARNAC_FORK", "1")
            .stdin(Stdio::piped())
            .spawn() {
        Err(why) => { panic!("err: {}", why.description()) },
        Ok(p) => p,
    };

    // Read 4k from the real pipe
    let mut buffer: [u8; 4096] = [0; 4096];
    let stdin = io::stdin();
    let len;
    {
        let mut sin = stdin.lock();
        len = sin.read(&mut buffer).unwrap();
    }

    println!("========================================");

    let buf = &buffer[0..len];
    hash_and_save(&buf, true);

    let mut new_io = buf.chain(stdin.lock());

    // process.stdin.unwrap()
    //     .write_all(new_io)
    //     // .write_all(&buf[0..len])
    //     .unwrap_or_else(|e| { panic!("{}", e) });

    let mut child_in = process.stdin.unwrap();
    match io::copy(&mut new_io, &mut child_in) {
        // TODO handle broken pipes
        Err(why) => panic!("copy err: {}", why.description()),
        Ok(_) => {},
    }

    // let mut s = String::new();
    // match process.stdout.unwrap().read_to_string(&mut s) {
    //     Err(why) => panic!("couldn't read stdout: {}", why.description()),
    //     Ok(_) => print!("stdout:\n{}", s),
    // };
}

fn main() {
    real_main()
}
