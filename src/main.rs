extern crate notify;

use notify::{RecommendedWatcher, Watcher, Event};
use std::sync::mpsc::channel;
use std::sync::mpsc::Receiver;
use std::env::args;
use std::process::exit;
use std::io;
use std::fs::OpenOptions;
use std::io::Read;
use std::iter::Fuse;
use std::iter::Iterator;

///
/// filewatch: Watches a file and provides information when it changes
///

fn main() {
    let mut args = args();
    if args.len() != 2 {
        println!("Please provide the name of one file to watch");
        exit(-3);
    }
    let file_name = args.nth(1).unwrap();

    let (tx, rx) = channel();
    let w: Result<RecommendedWatcher, notify::Error> = Watcher::new(tx);
    match w {
        Ok(mut watcher) => {
            match watcher.watch(&file_name) {
                Ok(()) => {
                    watch_loop(rx, &file_name);
                },
                Err(e) => {
                    println!("Could not watch file: {:?}", e);
                },
            }
        },
        Err(e) => {
            println!("Could not create a filesystem watcher: {:?}", e);
            exit(-2);
        },
    }
}

fn watch_loop(rx: Receiver<Event>, file_name: &str) {
    let mut file_content: Vec<u8> = Vec::new();
    match OpenOptions::new().read(true).write(false).open(file_name) {
        Ok(mut file) => {
            match file.read_to_end(&mut file_content) {
                Err(e) => {
                    println!("Could not read file {}: {:?}", file_name, e);
                    exit(-4);
                },
                _ => {},
            }
        },
        Err(e) => {
            println!("Could not open file {}: {:?}", file_name, e);
            exit(-4);
        }
    }

    loop {
        match rx.recv() {
            Ok(event) => {
                println!("Received event: {:?}", event);
                file_content = match check_changes(&file_content, file_name) {
                    Ok(new_content) => new_content,
                    Err(e) => {
                        println!("Could not read file {}: {:?}", file_name, e);
                        exit(-4);
                    }
                };
            },
            Err(e) => {
                println!("Event receive error: {:?}", e);
                exit(-4);
            }
        }
    }
}

fn check_changes(old_content: &[u8], file_name: &str) -> io::Result<Vec<u8>> {
    let mut new_content: Vec<u8> = Vec::with_capacity(old_content.len());

    match OpenOptions::new().read(true).write(false).open(file_name) {
        Ok(mut file) => {
            match file.read_to_end(&mut new_content) {
                Ok(_) => {
                    dump_changes(old_content, &new_content);
                    Ok(new_content)
                },
                Err(e) => Err(e),
            }
        },
        Err(e) => Err(e),
    }
}

fn dump_changes(old: &[u8], new: &[u8]) {
    println!("-------------------------------------");
    if old.len() != new.len() {
        println!("File size changed from {} to {}", old.len(), new.len());
    }

    // Create an iterator that associates an index with the old and new values
    let iter = OptionZip::new(old.iter(), new.iter()).enumerate();
    for (index, (old_byte, new_byte)) in iter {
        if old_byte != new_byte {
            match (old_byte, new_byte) {
                (Some(_), None) => println!("Byte {} deleted", index),
                (None, Some(new_value)) => println!("Byte {} added: 0x{:x}", index, new_value),
                (Some(old_value), Some(new_value)) => {
                    println!("Byte {} changed from 0x{:x} to 0x{:x}", index, old_value, new_value);
                },
                _ => {},
            }
        }
    }
}

/// An iterator that behaves similarly to std::iter::Zip, but continues providing values
/// until both contained iterators return None.
struct OptionZip<A: Iterator, B: Iterator> {
    iter_a: Fuse<A>,
    iter_b: Fuse<B>,
}

impl<A: Iterator, B: Iterator> OptionZip<A, B> {
    pub fn new(iter_a: A, iter_b: B) -> OptionZip<A, B> {
        OptionZip {
            iter_a: iter_a.fuse(),
            iter_b: iter_b.fuse(),
        }
    }
}

impl<A: Iterator, B: Iterator> Iterator for OptionZip<A, B> {
    type Item = (Option<A::Item>, Option<B::Item>);
    fn next(&mut self) -> Option<Self::Item> {
        match (self.iter_a.next(), self.iter_b.next()) {
            (None, None) => None,
            (a, b) => Some((a, b)),
        }
    }
}
