# kinode-python

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
echo 'import sys; from pprint import pprint as pp; pp(sys.path); pp(sys.platform); raise Exception("oops"); pp(sys.platform)' > test.py
kit i python:python:sys '"Run"' -b test.py
```

Doesn't currently work!
Do the above with Kinode on full event log mode (Ctrl+V 3 times) to see why: Response doesn't get routed properly.
