use serde::{Serialize, Deserialize};

// python Requests and Responses encode the Request `code`
//  and the Response `output` in the `lazy_load_blob`
#[derive(Debug, Serialize, Deserialize)]
pub enum PythonRequest {
    Run,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum PythonResponse {
    Run,
    Err(String),
}
