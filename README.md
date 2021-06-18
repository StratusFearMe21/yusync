# YuSync

A syncing agent for [YuPass](https://github.com/StratusFearMe21/yupass), or really any file on your computer

## Setup

These commands build and start the server on port 8080

```bash
cargo build --release
cargo run
```

## Usage

The intent of this project was to provide syncing for the [YuPass](https://github.com/StratusFearMe21/yupass) Password manager, but it can be used for other syncing tasks as well using it's API

*   **GET /rev** returns the last time that the file was revised, it is incremented by 1 each time the file is changed
*   **GET /download** downloads the file being synced
*   **POST /upload** replaces the file on the server with a new one and increments **rev** by 1.
