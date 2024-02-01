use clap::Parser;
use futures_util::{SinkExt, StreamExt};
use tokio::sync::mpsc;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message::{Binary, Close}};
use wasi_common::pipe::{ReadPipe, WritePipe};
use wasmtime::{Config, Engine, Linker, Module, Store};
use wasmtime_wasi::WasiCtxBuilder;

use kinode_types::HttpServerAction;

type Receiver = mpsc::Receiver<Vec<u8>>;
type Sender = mpsc::Sender<Vec<u8>>;

/// Kinode Python code runner extension
#[derive(Parser, Debug)]
#[command(author, version, about)]
struct Args {
    /// Kinode port
    #[arg(short, long)]
    port: u16,
}

include!("python_includes.rs");

const LOCALHOST: &str = "ws://localhost";
const PROCESS_ID: &str = "python:python:sys";
const EVENT_LOOP_CHANNEL_CAPACITY: usize = 100;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let url = format!("{}:{}/{}", LOCALHOST, args.port, PROCESS_ID);
    let (send_to_loop, mut recv_in_loop): (Sender, Receiver) = mpsc::channel(
        EVENT_LOOP_CHANNEL_CAPACITY
    );
    let (ws_stream, _) = connect_async(&url).await.expect("Failed to connect");
    let (mut write, mut read) = ws_stream.split();

    loop {
        tokio::select! {
            Some(message) = read.next() => {
                match message {
                    Ok(Binary(ref request)) => {
                        let request = rmp_serde::from_slice(request)?;
                        python(request, send_to_loop.clone()).await?;
                    }
                    Ok(Close(_)) => {
                        eprintln!("Server closed the connection");
                        return Err(anyhow::anyhow!("Server closed the connection"));
                    }
                    Err(e) => {
                        eprintln!("Error in receiving message: {}", e);
                        return Err(anyhow::anyhow!("Error in receiving message: {}", e));
                    }
                    _ => {}
                }
            }
            Some(result) = recv_in_loop.recv() => {
                match write.send(Binary(result)).await {
                    Ok(_) => { println!("sending result"); }
                    Err(e) => {
                        eprintln!("Error in sending message: {}", e);
                    }
                }
            }
        }
    }
}

async fn python(
    request: HttpServerAction,
    send_to_loop: Sender,
) -> anyhow::Result<()> {
    let HttpServerAction::WebSocketExtPushData { id, kinode_message_type, blob } = request else {
        return Err(anyhow::anyhow!(""));
    };
    let code = String::from_utf8(blob)?;
    let send_to_loop = send_to_loop.clone();
    tokio::spawn(async move {
        let result = run_python(&code).await.unwrap();
        println!("got\n{:?}\nfrom\n{}", String::from_utf8(result.clone()), code);
        let result = rmp_serde::to_vec(&HttpServerAction::WebSocketExtPushData {
            id,
            kinode_message_type,
            blob: result,
        }).unwrap();
        let _ = send_to_loop.send(result).await;
    });
    Ok(())
}

async fn run_python(code: &str) -> anyhow::Result<Vec<u8>> {
    let wasi_stdin = ReadPipe::from(code);
    let wasi_stdout = WritePipe::new_in_memory();
    let wasi_stderr = WritePipe::new_in_memory();

    let result = {
        // Define the WASI functions globally on the `Config`.
        let mut config = Config::new();
        config.async_support(true);
        let engine = Engine::new(&config)?;
        let mut linker = Linker::new(&engine);
        wasmtime_wasi::add_to_linker(&mut linker, |s| s)?;

        // uncomment to bring in non-stdlib libs (except those containing C code)
        // let dir = cap_std::fs::Dir::open_ambient_dir(
        //     "venv",
        //     cap_std::ambient_authority(),
        // ).unwrap();

        // Create a WASI context and put it in a Store; all instances in the store
        // share this context. `WasiCtxBuilder` provides a number of ways to
        // configure what the target program will have access to.
        let wasi = WasiCtxBuilder::new()
            // uncomment to bring in non-stdlib libs (except those containing C code)
            // .preopened_dir(dir, "/venv")?
            // .env("PYTHONPATH", "/venv/lib/python3.12/site-packages")?
            .stdin(Box::new(wasi_stdin.clone()))
            .stdout(Box::new(wasi_stdout.clone()))
            .stderr(Box::new(wasi_stderr.clone()))
            .build();
        let mut store = Store::new(&engine, wasi);

        // Instantiate our module with the imports we've created, and run it.
        let module = Module::from_binary(&engine, PYTHON_WASM)?;
        linker.module_async(&mut store, "", &module).await?;

        linker
            .get_default(&mut store, "")?
            .typed::<(), ()>(&store)?
            .call_async(&mut store, ())
            .await
    };

    let contents: Vec<u8> = match result {
        Ok(_) => wasi_stdout
            .try_into_inner()
            .expect("sole remaining reference to WritePipe")
            .into_inner(),
        Err(_) => wasi_stderr
            .try_into_inner()
            .expect("sole remaining reference to WritePipe")
            .into_inner(),
    };

    Ok(contents)
}
