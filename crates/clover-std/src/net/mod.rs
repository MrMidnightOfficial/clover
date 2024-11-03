use clover::{State, Object, NativeModel};
use clover::debug::RuntimeError;
use std::rc::Rc;
use std::cell::RefCell;
use tokio::runtime::Runtime; // Add this import

use reqwest;


#[derive(Debug)]
pub struct Net;

impl NativeModel for Net {
    fn model_get(&self, key: &str) -> Result<Object, RuntimeError> {
        match key {
            "get" => Ok(Object::NativeFunction(sync_get)),
            "has_wifi" => Ok(Object::NativeFunction(has_wifi)),
            _ => Ok(Object::Null)
        }
    }
}


// synchronous wrapper function
fn sync_get(state: &mut State, parameters: &[Object]) -> Result<Object, RuntimeError> {
    let rt = Runtime::new().unwrap();
    rt.block_on(async_get(state, parameters))
}

fn has_wifi(state: &mut State, parameters: &[ Object ]) -> Result<Object, RuntimeError> {
    let test_url = "http://www.google.com";
    let parameters = vec![Object::String(Rc::new(RefCell::new(test_url.to_string())))];
    match sync_get(state, &parameters) {
        Ok(object) => Ok(Object::Boolean(true)),
        Err(e) => Ok(Object::Boolean(false)),
    }
}

pub async fn async_get(state: &mut State, parameters: &[ Object ]) -> Result<Object, RuntimeError> {
    if parameters.len() != 1 {
        return Err(RuntimeError::new("Expected exactly one parameter", state.last_position()));
    }

    let url = match &parameters[0] {
        Object::String(url) => url.clone(),
        _ => return Err(RuntimeError::new("Expected a string as the first parameter", state.last_position()))
    };

    // Simulate a network request
    let url_borrowed = url.borrow();
    let response = reqwest::get(&*url_borrowed).await.map_err(|e| {
        RuntimeError::new(&format!("Network request failed: {}", e), state.last_position())
    })?;
    let status = response.status(); // Store status before moving response
    let response_text = response.text().await.map_err(|e| {
        RuntimeError::new(&format!("Failed to read response body: {}", e), state.last_position())
    })?;

    println!("Status: {}", status); // Use the stored status
    println!("Body: {}", response_text);

    Ok(Object::String(Rc::new(RefCell::new(response_text))))
}
