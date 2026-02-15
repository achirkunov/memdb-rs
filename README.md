# memdb-rs

[![CI](https://github.com/achirkunov/memdb-rs/actions/workflows/ci.yml/badge.svg)](https://github.com/achirkunov/memdb-rs/actions/workflows/ci.yml)

A toy in-memory key-value store that implements a subset of the Redis RESP protocol. Built as a learning project.

## Supported Commands

| Command | Syntax | Description |
|---------|--------|-------------|
| PING | `PING [message]` | Returns PONG, or echoes the message |
| ECHO | `ECHO message` | Echoes the message |
| SET | `SET key value [EX seconds]` | Sets a key to a value, with optional TTL |
| GET | `GET key` | Returns the value for a key |
| DEL | `DEL key [key ...]` | Deletes one or more keys |
| COMMAND | `COMMAND` | Returns OK |

## Persistence

Append-only file (AOF) persistence can be enabled with the `--appendonly` flag. When enabled, every write command (SET, DEL) is logged in RESP format and fsynced to disk. On startup, the AOF file is replayed to rebuild the dataset.

AOF is disabled by default.

## Running

```sh
cargo run --bin memdb-server
```

Options:

```
--port, -p <port>     (default: 6379)
--bind, -b <addr>     (default: 127.0.0.1)
--appendonly          Enable AOF persistence
--aof-file <path>     AOF file path (default: appendonly.aof)
```

## Testing

### Installing redis-cli

**macOS (Homebrew):**

```sh
brew install redis
```

This installs the full Redis package, but you only need the `redis-cli` binary. No need to start the Redis server.

### Using redis-cli

```sh
redis-cli PING
# => PONG

redis-cli SET foo bar
# => OK
```

### Using netcat

PING:

```sh
echo -e '*1\r\n$4\r\nPING\r\n' | nc 127.0.0.1 6379
```

SET foo bar:

```sh
echo -e '*3\r\n$3\r\nSET\r\n$3\r\nfoo\r\n$3\r\nbar\r\n' | nc 127.0.0.1 6379
```
