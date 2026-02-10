# memdb-rs

An in-memory key-value store that speaks the Redis RESP protocol.

## Running

```sh
cargo run --bin memdb-server
```

Options:

```
--port, -p <port>   (default: 6379)
--bind, -b <addr>   (default: 127.0.0.1)
```

## Installing redis-cli

**macOS (Homebrew):**

```sh
brew install redis
```

This installs the full Redis package, but you only need the `redis-cli` binary. No need to start the Redis server.

## Testing

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
