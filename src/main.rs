// Copyright 2016 Leo Schwarz <mail@leoschwarz.com>
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

extern crate chrono;
extern crate clap;
extern crate iron;
#[macro_use]
extern crate log;
extern crate env_logger;

use std::io;
use std::io::prelude::*;
use std::io::BufReader;
use std::fs::File;
use std::process::Command;

use chrono::*;
use iron::prelude::*;
use iron::status;

const BAD_REQ_MESSAGE: &'static str = "Check documentation for instructions.";

/// Only volumes recognized in a whitelist can be snapshotted.
///
/// Formatting:
///  • empty lines are ignored
///  • lines starting with "#" are ignored.
///
/// Pro/cons of whitelist:
/// (+) information leakage about volumes not in the list is impossible
/// (+) input sanitation simplified
/// (-) need to specify a whitelist → less flexibility
struct VolumeWhitelist {
    volumes: Vec<String>
}

impl VolumeWhitelist {
    fn len(&self) -> usize {
        self.volumes.len()
    }

    fn is_allowed(&self, path: &String) -> bool {
        self.volumes.contains(path)
    }

    fn read<'a>(location: &'a str) -> Result<VolumeWhitelist, io::Error> {
        let file = BufReader::new(try!(File::open(location)));
        let mut volumes = Vec::<String>::new();

        for line in file.lines() {
            match line {
                Ok(line) => {
                    let volume = line.trim();
                    if !volume.is_empty() && volume.chars().nth(0).unwrap() != '#' {
                        volumes.push(line.trim().to_string());
                    }
                },
                _ => {}
            }
        }
        Ok(VolumeWhitelist {
            volumes: volumes
        })
    }
}

/// Performs a snapshot, input validation assumed.
fn perform_snapshot(path: &String) -> Result<(), io::Error> {
    // Generate snapshot version name.
    let snapshot_version = Local::now().format("%Y-%m-%d");
    info!("Creating snapshot {}@{}", path, snapshot_version);

    // Create the snapshot.
    let status = try!(
        Command::new("zfs")
            .arg("snapshot")
            .arg(format!("{}@{}", path, snapshot_version))
            .status()
    );

    if status.success() {
        info!("Created snapshot {}@{} successfully.", path, snapshot_version);
        Ok(())
    } else {
        error!("Failed creating snapshot {}@{}", path, snapshot_version);
        Err(io::Error::new(io::ErrorKind::Other, "Snapshot failed."))
    }
}

fn main() {
    // Setup logger.
    env_logger::init();

    // Parse arguments.
    let args = clap::App::new("snapshot_manager")
        .version("0.1.0")
        .author("Leo Schwarz <mail@leoschwarz.com>")
        .about("Manager for ZFS snapshots")
        .arg(clap::Arg::with_name("port")
            .short("p")
            .long("port")
            .value_name("PORT")
            .help("Server port")
            .takes_value(true)
        )
        .arg(clap::Arg::with_name("host")
            .short("h")
            .long("host")
            .value_name("HOST")
            .help("Server host")
        )
        .arg(clap::Arg::with_name("whitelist")
            .short("w")
            .long("whitelist")
            .value_name("WHITELIST")
            .help("Whitelist location")
            .required(true)
        )
        .get_matches();

    // Parse whitelist.
    let whitelist_path = args.value_of("whitelist").unwrap();
    let whitelist = VolumeWhitelist::read(whitelist_path).unwrap();
    info!("Whitelist containing {} elements read from system.", whitelist.len());

    // Setup server.
    let server_host = args.value_of("host").unwrap_or("localhost");
    let server_port = args.value_of("port").unwrap_or("7877").parse::<u16>().unwrap();
    info!("Starting server on {}:{}", server_host, server_port);

    Iron::new(move |req: &mut Request| {
        match req.url.query() {
            Some(query) => {
                info!("Request query: {}", query);

                // Check if snapshotting of volume is allowed.
                let path = query.trim().to_string();
                if whitelist.is_allowed(&path) {
                    match perform_snapshot(&path) {
                        Ok(_) => Ok(Response::with((status::Ok, "Snapshot successful."))),
                        _ => Ok(Response::with((status::InternalServerError, "Snapshot failed.")))
                    }
                } else {
                    Ok(Response::with((status::BadRequest, BAD_REQ_MESSAGE)))
                }
            },
            None => {
                debug!("Request without query encountered.");

                Ok(Response::with((status::BadRequest, BAD_REQ_MESSAGE)))
            }
        }
    }).http((server_host, server_port)).unwrap();
}
