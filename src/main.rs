// Copyright [2022] [Mark Benvenuto]
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

// use std::collections::HashMap;
// use std::ffi::OsString;

use anyhow::Result;
use clap::{ArgEnum, Parser};
use human_panic::setup_panic;
use serde_derive::{Deserialize, Serialize};

mod types;
use types::CommonProcInfo;

#[cfg(target_os = "linux")]
mod linux;

#[cfg(target_os = "macos")]
mod macos;
use macos::get_procs;

// If we derive our own ArgEnum, we can get better case
// Because ArgEnum default case conversion converts "_" to "-" and CamelCase to "camel-case"
#[derive(Debug, PartialEq, Clone, ArgEnum)]
enum MongoProcess {
    Legacyshell,
    Mongod,
    Mongos,
    // Mongoq,
    // TODO
}

fn is_mongo_process(proc: &CommonProcInfo) -> Option<MongoProcess> {
    if !proc.program.starts_with("mongo") {
        return Option::None;
    }

    if proc.program == "mongod" {
        Some(MongoProcess::Mongod)
    } else if proc.program == "mongos" {
        Some(MongoProcess::Mongos)
    } else if proc.program == "mongo" {
        Some(MongoProcess::Legacyshell)
    } else {
        eprintln!("Unexpected mongo like process found: {:?}", proc);
        None
    }
}

#[derive(Serialize, Deserialize, Debug, ArgEnum, Clone, PartialEq)]
enum MongoDType {
    Standalone,
    ReplicaSet,
    Config,
    Shard,
}

#[derive(Debug, ArgEnum, Clone, PartialEq)]
enum ReplicaSetType {
    Primary,
    Secondary,
}

#[derive(Serialize, Deserialize, Debug)]
struct MongoSServerInfo {
    pid: i32,
    port: i32,
}

#[derive(Serialize, Deserialize, Debug)]
struct MongoDServerInfo {
    pid: i32,
    port: i32,
    server_type: MongoDType,
    replica_set_name: Option<String>,
}

fn get_cmd_line_option(option: &str, options: &Vec<String>) -> Option<String> {
    for (i, opt) in options.iter().enumerate() {
        if opt == option {
            if i + 1 < options.len() {
                // NOTE: assume options are generally correct
                return Some(options[i + 1].to_owned());
            } else {
                return None;
            }
        } else if opt.starts_with(option) {
            let split = opt.split('=');
            let splits: Vec<&str> = split.collect();
            if splits.len() == 2 && splits[0] == option {
                return Some(splits[1].to_owned());
            }
        }
    }

    None
}

fn get_mongod_info(proc: &CommonProcInfo) -> MongoDServerInfo {
    assert_eq!(is_mongo_process(proc), Some(MongoProcess::Mongod));

    let cmdline: &Vec<String> = &proc.cmdline;
    let port_str = get_cmd_line_option("--port", cmdline);
    let port = port_str.map_or(20017, |s| s.parse::<i32>().expect("Bad port number"));

    let shardsvr = get_cmd_line_option("--shardsvr", cmdline);
    let configsvr = get_cmd_line_option("--configsvr", cmdline);

    let repl_set = get_cmd_line_option("--replSet", cmdline);

    let mut server_type = MongoDType::Standalone;
    if configsvr.is_some() {
        server_type = MongoDType::Config;
    } else if shardsvr.is_some() {
        server_type = MongoDType::Shard;
    } else if repl_set.is_some() {
        server_type = MongoDType::ReplicaSet;
    }

    MongoDServerInfo {
        pid: proc.pid,
        port,
        server_type,
        replica_set_name: repl_set,
    }
}

fn get_mongos_info(proc: &CommonProcInfo) -> MongoSServerInfo {
    assert_eq!(is_mongo_process(proc), Some(MongoProcess::Mongos));

    let cmdline: &Vec<String> = &proc.cmdline;
    let port_str = get_cmd_line_option("--port", cmdline);
    let port = port_str.map_or(20017, |s| s.parse::<i32>().expect("Bad port number"));

    MongoSServerInfo {
        pid: proc.pid,
        port,
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct MongoPSInfo {
    mongod: Vec<MongoDServerInfo>,
    mongos: Vec<MongoSServerInfo>,
    shell: Vec<i32>,
}

// Simple process picker for mongodb development
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// Process Type
    #[clap(short = 't', long = "type", arg_enum, value_parser)]
    process_type: Option<MongoProcess>,

    /// ServerType
    #[clap(long, arg_enum, value_parser)]
    server_type: Option<MongoDType>,

    /// Port of mongo daemon to search for
    #[clap(short, long)]
    port: Option<i32>,

    /// Verbose
    #[clap(short, long)]
    verbose: bool,
}

fn main() -> Result<()> {
    setup_panic!();

    let args = Args::parse();

    // println!("args: {:?}", args);

    if let Some(ref pt) = args.process_type {
        if *pt == MongoProcess::Legacyshell && args.port.is_some() {
            eprintln!("ERROR: Cannot use port with legacy shell");
            std::process::exit(1);
        }
    }

    // Get a list of processes
    let procs = get_procs()?;

    let mut shells: Vec<i32> = Vec::new();

    let mut mongod_servers: Vec<MongoDServerInfo> = Vec::new();
    let mut mongos_servers: Vec<MongoSServerInfo> = Vec::new();

    // Get a list of mongodb information
    for p in procs {
        let mp = is_mongo_process(&p);
        if mp.is_some() && args.verbose {
            println!("{:?} -{:?} -{:?} -{:?}", p.pid, mp, p.program, p.cmdline);
        }

        if let Some(mpt) = mp {
            match mpt {
                MongoProcess::Legacyshell => {
                    shells.push(p.pid);
                }
                MongoProcess::Mongod => {
                    mongod_servers.push(get_mongod_info(&p));
                }
                MongoProcess::Mongos => {
                    mongos_servers.push(get_mongos_info(&p));
                }
            }
        }
    }

    // Dump Process Info
    if args.verbose {
        for s in shells.as_slice() {
            println!("Shell: {:?}", s);
        }
        for d in mongod_servers.as_slice() {
            println!("{:?}", d);
        }
        for s in mongos_servers.as_slice() {
            println!("{:?}", s);
        }
    }

    // Find servers by port
    let mut pids: Vec<i32> = Vec::new();
    let mut has_pids = true;
    if let Some(port) = args.port {
        // TODO - nightly's iter_collect_into would be nice here
        pids.extend_from_slice(
            mongod_servers
                .as_slice()
                .iter()
                .filter(|d| d.port == port)
                .map(|d| d.pid)
                .collect::<Vec<i32>>()
                .as_slice(),
        );
        pids.extend_from_slice(
            mongos_servers
                .as_slice()
                .iter()
                .filter(|d| d.port == port)
                .map(|d| d.pid)
                .collect::<Vec<i32>>()
                .as_slice(),
        );
    } else if let Some(server_type) = args.server_type {
        pids.extend_from_slice(
            mongod_servers
                .as_slice()
                .iter()
                .filter(|d| d.server_type == server_type)
                .map(|d| d.pid)
                .collect::<Vec<i32>>()
                .as_slice(),
        );
    } else if let Some(process_type) = args.process_type {
        match process_type {
            MongoProcess::Legacyshell => {
                pids.extend_from_slice(shells.as_slice());
            }
            MongoProcess::Mongod => {
                pids.extend_from_slice(
                    mongod_servers
                        .as_slice()
                        .iter()
                        .map(|d| d.pid)
                        .collect::<Vec<i32>>()
                        .as_slice(),
                );
            }
            MongoProcess::Mongos => {
                pids.extend_from_slice(
                    mongos_servers
                        .as_slice()
                        .iter()
                        .map(|d| d.pid)
                        .collect::<Vec<i32>>()
                        .as_slice(),
                );
            }
        }
    } else {
        has_pids = false;
    }

    if has_pids {
        for pid in pids {
            println!("{}", pid)
        }
    } else {
        // If there were no filters, dump all the process info as json
        let summary = MongoPSInfo {
            shell: shells,
            mongod: mongod_servers,
            mongos: mongos_servers,
        };
        println!("{}", serde_json::to_string_pretty(&summary)?);
    }

    Ok(())
}

#[test]
fn test_cmd_opts() {
    let opts1 = vec!["foo".to_owned()];
    assert_eq!(get_cmd_line_option("--port", &opts1), None);

    let opts2 = vec!["--port".to_owned(), "20000".to_owned()];
    assert_eq!(
        get_cmd_line_option("--port", &opts2),
        Some("20000".to_owned())
    );

    let opts2a = vec!["--port".to_owned()];
    assert_eq!(get_cmd_line_option("--port", &opts2a), None);

    let opts3 = vec!["--port=20000".to_owned()];
    assert_eq!(
        get_cmd_line_option("--port", &opts3),
        Some("20000".to_owned())
    );

    let opts3a = vec!["--ports=20000".to_owned()];
    assert_eq!(get_cmd_line_option("--port", &opts3a), None);
}
