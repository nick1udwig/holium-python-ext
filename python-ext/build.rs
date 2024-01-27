// use std::process::Command;
use std::{fs, io, io::Write};

const PYTHON_WASM_FILE_NAME: &str = "python-3.12.0.wasm";
const PYTHON_WASM_URL: &str = "https://github.com/vmware-labs/webassembly-language-runtimes/releases/download/python%2F3.12.0%2B20231211-040d5a6";

// fn run_command(cmd: &mut Command) -> io::Result<()> {
//     let status = cmd.status()?;
//     if status.success() {
//         Ok(())
//     } else {
//         Err(io::Error::new(io::ErrorKind::Other, "Command failed"))
//     }
// }

async fn get_python_wasm(url: &str) -> anyhow::Result<String> {
    let pwd = std::env::current_dir().unwrap();
    let path = format!("{}/target/{}", pwd.display(), PYTHON_WASM_FILE_NAME);
    if !std::path::Path::new(&path).exists() {
        let response = reqwest::get(url).await?;
        if !response.status().is_success() {
            return Err(anyhow::anyhow!("couldn't get python.wasm"));
        } else {
            let content = response.bytes().await?;
            let mut file = fs::File::create(&path)?;
            file.write_all(&content)?;
        }
    }
    Ok(path)
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    if std::env::var("SKIP_BUILD_SCRIPT").is_ok() {
        println!("Skipping build script");
        return Ok(());
    }

    let pwd = std::env::current_dir().unwrap();

    // Pull wit from git repo
    let wit_dir = pwd.join("wit");
    fs::create_dir_all(&wit_dir).unwrap();
    let wit_file = wit_dir.join("kinode.wit");
    if !wit_file.exists() {
        // TODO: cache in better way
        let mut wit_file = std::fs::File::create(&wit_file).unwrap();
        let kinode_wit_url =
            "https://raw.githubusercontent.com/uqbar-dao/kinode-wit/master/kinode.wit";
        let mut response = reqwest::blocking::get(kinode_wit_url).unwrap();
        io::copy(&mut response, &mut wit_file).unwrap();
    }

    // // Create target.wasm (compiled .wit) & world
    // run_command(Command::new("wasm-tools").args([
    //     "component",
    //     "wit",
    //     &format!("{}/wit/", pwd.display()),
    //     "-o",
    //     "target.wasm",
    //     "--wasm",
    // ]))
    // .unwrap();
    // run_command(Command::new("touch").args([&format!("{}/world", pwd.display())])).unwrap();

    // Get python.wasm & include it
    let mut python_includes =
        fs::File::create(format!("{}/src/python_includes.rs", pwd.display())).unwrap();
    let python_wasm_file_path =
        get_python_wasm(&format!("{}/{}", PYTHON_WASM_URL, PYTHON_WASM_FILE_NAME)).await?;
    writeln!(
        python_includes,
        "pub static PYTHON_WASM: &[u8] = include_bytes!(\"{}\");",
        python_wasm_file_path,
    )
    .unwrap();

    Ok(())
}
