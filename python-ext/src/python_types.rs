use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize)]
pub enum PythonRequest {
    RunScript {
        package_id: String,
        /// The scripts requirements.txt
        requirements: String,
        /// The script to run must be in the package's `scripts` directory
        script: String,
        /// The function to call in the script
        func: String,
        /// The arguments to pass to the script
        args: Vec<String>,
    },
}

// python Responses encode the `output` in the `lazy_load_blob`
#[derive(Debug, Serialize, Deserialize)]
pub enum PythonResponse {
    RunScript,
    Err(String),
}
