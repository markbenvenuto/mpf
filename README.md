# Mongo Process Finder

Enumerates the local mongod, mongos and related processes with optional filtering. Designed to be used to help find the right process to debug.

## Options

```
mpf 0.1.0

USAGE:
    mps [OPTIONS]

OPTIONS:
    -h, --help                         Print help information
    -p, --port <PORT>                  Port of mongo daemon to search for
        --server-type <SERVER_TYPE>    ServerType [possible values: standalone, replica-set, config,
                                       shard]
    -t, --type <PROCESS_TYPE>          Process Type [possible values: legacyshell, mongod, mongos]
    -v, --verbose                      Verbose
    -V, --version                      Print version information
```


## Build
Get Rust from https://rustup.rs/.

```cargo build --release```

## License

Apache 2.0

## Roadmap

- add support for filter by replica set name
- add support for filter by replica set role - i.e. is primary or secondary by connecting to mongod

- investigate cargo-dist

- add python wrapper so lldb can directly call it