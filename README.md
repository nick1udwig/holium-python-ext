# python

The Kinode python-ext and python process.

## Usage

ORDER MATTERS

python process MUST be started before python-ext

```
# Terminal 1: run Kinode
# Use `develop` branch of Kinode
kit f -r ~/path/to/kinode

# Terminal 2: build & start python process
cd python
kit bs

# Terminal 3: build & start python-ext
cd python-ext
cargo run --release -- --port 8080

# Send a command
# TODO
```
