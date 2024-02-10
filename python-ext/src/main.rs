use clap::Parser;
use futures_util::{SinkExt, StreamExt};
use std::path::PathBuf;
use tokio::fs;
use tokio::sync::mpsc;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message::{Binary, Close}};

use kinode_lib::types::http_server::HttpServerAction;

mod python_types;
use python_types::PythonRequest;

/// A python module that provides a python interface to Kinode processes.
/// This module is implemented in Rust using the PyO3 library.
use pyo3::prelude::*;
use pyo3::types::{PyString, PyTuple};

use std::process::Command;

type Receiver = mpsc::Receiver<Vec<u8>>;
type Sender = mpsc::Sender<Vec<u8>>;

/// Kinode Python code runner extension
#[derive(Parser, Debug)]
#[command(author, version, about)]
struct Args {
    /// Kinode port
    #[arg(short, long)]
    port: u16,

    /// Kinode home directory
    #[arg(short, long)]
    home: String,
}

const LOCALHOST: &str = "ws://localhost";
const PROCESS_ID: &str = "python:python:holium.os";
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
                        println!("got Request");
                        let request = rmp_serde::from_slice(request)?;
                        python(args.home.clone(), request, send_to_loop.clone()).await?;
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
    home: String,
    request: HttpServerAction,
    send_to_loop: Sender,
) -> anyhow::Result<()> {
    println!("processing Request");
    let HttpServerAction::WebSocketExtPushData { id, kinode_message_type, blob } = request else {
        return Err(anyhow::anyhow!("not a WebSocketExtPushData, as expected"));
    };
    println!("still processing Request");
    let PythonRequest::RunScript { package_id, requirements, script, func, args } = rmp_serde::from_slice(&blob)?;
    println!("got Request: package_id, requirements, script, func, args: {:?}, {:?}, {:?}, {:?}, {:?}", package_id, requirements, script, func, args);
    tokio::spawn(async move {
        let result = run_python(&home, &package_id, &requirements, &script, &func, args).await.unwrap();
        println!("got\n{}", result);
        let result = rmp_serde::to_vec(&HttpServerAction::WebSocketExtPushData {
            id,
            kinode_message_type,
            blob: result.into_bytes(),
        }).unwrap();
        let _ = send_to_loop.send(result).await;
    });
    Ok(())
}

async fn install_requirements(requirements_path: PathBuf) -> anyhow::Result<()> {
    //let path = Path::new(requirements_path);

    // Read the file to a string
    let contents = fs::read_to_string(requirements_path).await?;

    // Split the contents by new line and iterate over each package
    for line in contents.lines() {
        // Skip empty lines or lines that start with '#' (comments)
        if line.trim().is_empty() || line.starts_with('#') {
            continue;
        }

        // Execute `pip3 install` for the current package
        let output = Command::new("pip3")
            .arg("install")
            .arg(line)
            .output()?;

        // Check if the command was executed successfully
        if output.status.success() {
            println!("Successfully installed: {}", line);
        } else {
            // Output the error if the installation failed
            eprintln!(
                "Error installing {}: {}",
                line,
                String::from_utf8_lossy(&output.stderr)
            );
        }
    }

    Ok(())
}

fn make_pkg_dir(home: &str, package_id: &str) -> PathBuf {
    PathBuf::from(home)
        .join("vfs")
        .join(package_id)
        .join("pkg")
}

fn make_full_path(home: &str, package_id: &str, path: &str) -> PathBuf {
    make_pkg_dir(home, package_id)
        .join("scripts")
        .join(path)
}

async fn run_python(
    home: &str,
    package_id: &str,
    requirements_path: &str,
    script_path: &str,
    func: &str,
    args: Vec<String>,
) -> anyhow::Result<String> {
    let requirements_path = make_full_path(home, package_id, requirements_path);
    let script_path = make_full_path(home, package_id, script_path);
    println!("{:?}, {:?}", requirements_path, script_path);
    install_requirements(requirements_path).await?;
    let script_contents = fs::read_to_string(script_path).await?;

    println!("running python");
    let package_path = make_pkg_dir(home, package_id);
    let response = Python::with_gil(|py| -> PyResult<_> {
        // set the current working directory to the package's directory
        let os = PyModule::import(py, "os")?;
        println!("python: chdir to: {}", package_path.display());
        println!("python: args: {}", args.join(", "));
        os.call_method1("chdir", (format!("{}", package_path.display()),))?;

        //let locals = [("os", py.import("os")?)].into_py_dict(py);
        //let script_name = script_name.split('.').next().unwrap();

        let py_args = PyTuple::new(
            py,
            &args.iter()
                .map(|arg| PyString::new(py, arg))
                .collect::<Vec<_>>(),
        );

        let function_result = PyModule::from_code(
            py,
            &script_contents,
            format!("{}.py", "script").as_str(),
            "script",
        )?
            .call_method1(
                func,
                py_args,
            )?
            .str()?
            .to_string();

        Ok(function_result)

        // let function_result: String = match py.run(&script, None, Some(locals)) {
        //     Ok(_) => {
        //         let module = PyModule::from_code(
        //             py,
        //             script.as_str(),
        //             format!("{}.py", script_name).as_str(),
        //             script_name,
        //         )
        //         .unwrap();

        //         let function = module.getattr(func.as_str()).unwrap();
        //         let py_args = PyTuple::new(
        //             py,
        //             &args
        //                 .iter()
        //                 .map(|arg| PyString::new(py, arg))
        //                 .collect::<Vec<_>>(),
        //         );

        //         let result = function.call1(py_args).unwrap();
        //         result.str().unwrap().to_string()
        //     }
        //     Err(e) => {
        //         println!("Failed to execute script: {:?}", e);
        //         e.to_string()
        //     }
        // };
        //Ok(function_result)
    })?;
    Ok(response)
}
