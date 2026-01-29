# Telegram Bot Logs

## Log Files

Logs are stored in the `logs/` directory:
- `logs/telegram-bot.log` - Main bot logs
- `logs/echo-bot.log` - Echo bot logs
- `logs/clock-bot.log` - Clock bot logs

## Log Format

```
[timestamp] [level] message
```

Example:
```
[2026-01-23 13:15:30.123] [INFO] Bot started at 2026-01-23 13:15:30
[2026-01-23 13:15:45.456] [INFO] [2026-01-23 13:15:45] [User:123456] [Chat:123456] Received: /start
[2026-01-23 13:15:45.789] [INFO] [2026-01-23 13:15:45] [User:123456] [Chat:123456] Sent welcome message
```

## Log Levels

- **TRACE** - Most detailed logging
- **DEBUG** - Debug information
- **INFO** - General informational messages
- **WARN** - Warning messages
- **ERROR** - Error messages

## Configuring Log Level

Set the `RUST_LOG` environment variable (e.g. in `.env` or when running):

```bash
# Debug mode (app crates only; framework stays at warn)
RUST_LOG=debug cargo run

# Info mode (default)
RUST_LOG=info cargo run

# Only errors
RUST_LOG=error cargo run
```

The logger automatically filters **Telegram bot framework** logs so they do not flood at debug/trace:
- `teloxide`, `reqwest`, `hyper`, `hyper_util` are set to **warn** in addition to your `RUST_LOG` level.
- To see framework debug (e.g. for teloxide), set explicitly: `RUST_LOG=debug,teloxide=debug`.

## Log Rotation

The logger appends to the existing log file. For production use, consider implementing log rotation:
- Size-based rotation (e.g., max 100MB per file)
- Time-based rotation (e.g., daily, weekly)
- Retention policy (e.g., keep last 7 days)
