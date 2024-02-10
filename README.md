# python

The Holium python-ext and python process.

## Setup

```
git clone git@github.com:hosted-fornet/holium-python-ext.git

git clone git@github.com:holium/memedeck.git
cd memedeck && git checkout hf/use-ext && cd ..

git clone git@github.com:kinode-dao/kinode.git
cd kinode && git checkout hf/holium-memedeck && cd ..
```

## Usage

ORDER MATTERS

python process MUST be started before python-ext

The paths below all assume your terminal & repos are structured based on the above [Setup](#setup) instructions

```
# Terminal 1: run Kinode
# ./kinode is path, assuming your terminal is pointing to the cloned & checkout'd repo you'd get from following the above
kit f -r ./kinode

# Terminal 2: build & start python process, then build & start python-ext
cd holium-python-ext
kit b python
kit s python

cd python-ext && cargo build --release && cd ..
# NOTE: replace port and home as appropriate
./python-ext/target/release/python --port 8080 --home /tmp/kinode-fake-node

# Start memedeck
# Terminal 3:
cd memedeck
kit b
kit s
```

## Connected repo/branches

https://github.com/kinode-dao/kinode/pull/244
https://github.com/kinode-dao/process_lib/pull/52
https://github.com/holium/memedeck/pull/13
https://github.com/hosted-fornet/holium-python-ext

## Current state of the repos:

1. Not using Python venv.
2. Loading memedeck spits out a bunch of errors that may be harmless?
   ```
   Error: Parse error: Failed to parse query at line 5 column 245 expected query to end
   ...Letterman's successor to the CBS late-night talk show Late Show.',
   perhaps missing a semicolon on the previous statement?
   ```
2. Memedeck hits interface process (python) and that request is passed to the extension properly.
3. The extension fails on my machine with
   ```
    thread 'tokio-runtime-worker' panicked at src/main.rs:95:96:
    called `Result::unwrap()` on an `Err` value: UnpicklingError: invalid load key, 'v'.
   ```
   which is the Python script vgg16.py failing at the unpickling step.

I took out the `pickle==4.0` from requirements.txt because my pip said it didn't do anything.
I'm not sure whether pickle failure is from my machine or a failed download of the file or what.

