use clover::{State, Object, NativeModel};
use clover::debug::RuntimeError;
use sysinfo::{System};
use std::env;

use std::rc::Rc;
use std::cell::RefCell
;
#[derive(Debug)]
pub struct Os;

impl NativeModel for Os {
    fn model_get(&self, key: &str) -> Result<Object, RuntimeError> {
        match key {
            "clock" => Ok(Object::NativeFunction(clock)),
            "get_os" => Ok(Object::NativeFunction(get_os)),
            "get_arch" => Ok(Object::NativeFunction(get_arch)),
            "get_total_memory" => Ok(Object::NativeFunction(get_total_memory)), // Returns the total available memory (RAM) in the system.
            "get_current_user" => Ok(Object::NativeFunction(get_current_user)),
            //"get_current_dir" => Ok(Object::NativeFunction(get_current_dir)),
            "is_file" => Ok(Object::NativeFunction(is_file)),
            "does_file_exist" => Ok(Object::NativeFunction(does_file_exist)),
            "does_dir_exist" => Ok(Object::NativeFunction(does_dir_exist)),
            "get_extension" => Ok(Object::NativeFunction(get_extension)),
            _ => Ok(Object::Null)
        }
    }
}


pub fn clock(_state: &mut State, _parameters: &[Object]) -> Result<Object, RuntimeError> {
    Ok(Object::Float(std::time::SystemTime::now().duration_since(std::time::SystemTime::UNIX_EPOCH).unwrap().as_secs_f64()))
}

pub fn get_os(_state: &mut State, _parameters: &[Object]) -> Result<Object, RuntimeError> {
    Ok(Object::String(std::rc::Rc::new(std::cell::RefCell::new(std::env::consts::OS.to_string()))))
}

pub fn get_arch(_state: &mut State, _parameters: &[Object]) -> Result<Object, RuntimeError> {
    Ok(Object::String(std::rc::Rc::new(std::cell::RefCell::new(std::env::consts::ARCH.to_string()))))
}

pub fn get_total_memory(_state: &mut State, _parameters: &[Object]) -> Result<Object, RuntimeError> {
    let mut sys = System::new_all();
    sys.refresh_all();

    Ok(Object::Float(sys.total_memory() as f64))
}

pub fn get_current_user(state: &mut State, _parameters: &[Object]) -> Result<Object, RuntimeError> {
    // Get the "USER" environment variable on Unix-like systems or "USERNAME" on Windows
    match env::var("USER").or_else(|_| env::var("USERNAME")) {
        Ok(username) => Ok(Object::String(std::rc::Rc::new(std::cell::RefCell::new(username)))),
        Err(error) => Err(RuntimeError::new(error.to_string().as_str(), state.last_position()))
    }
}

pub fn is_file(state: &mut State, parameters: &[Object]) -> Result<Object, RuntimeError> {
    if parameters.is_empty() {
        return Err(RuntimeError::new("No file path provided", state.last_position()));
    }

    let file_path = parameters[0].to_string();
    match std::fs::metadata(&file_path) {
        Ok(metadata) => Ok(Object::Boolean(metadata.is_file())),
        Err(error) => Err(RuntimeError::new(
            format!("Failed to check file '{}': {}", file_path, error).as_str(),
            state.last_position(),
        )),
    }
}

pub fn does_file_exist(state: &mut State, parameters: &[Object]) -> Result<Object, RuntimeError> {
    if parameters.is_empty() {
        return Err(RuntimeError::new("No file path provided", state.last_position()));
    }

    let file_path = parameters[0].to_string();
    match std::fs::metadata(&file_path) {
        Ok(_) => Ok(Object::Boolean(true)),
        Err(error) => {
            if error.kind() == std::io::ErrorKind::NotFound {
                Ok(Object::Boolean(false))
            } else {
                Err(RuntimeError::new(
                    format!("Failed to check file '{}': {}", file_path, error).as_str(),
                    state.last_position(),
                ))
            }
        }
    }
}

pub fn does_dir_exist(state: &mut State, parameters: &[Object]) -> Result<Object, RuntimeError> {
    if parameters.is_empty() {
        return Err(RuntimeError::new("No directory path provided", state.last_position()));
    }

    let dir_path = parameters[0].to_string();
    match std::fs::metadata(&dir_path) {
        Ok(metadata) => Ok(Object::Boolean(metadata.is_dir())),
        Err(error) => {
            if error.kind() == std::io::ErrorKind::NotFound {
                Ok(Object::Boolean(false))
            } else {
                Err(RuntimeError::new(
                    format!("Failed to check directory '{}': {}", dir_path, error).as_str(),
                    state.last_position(),
                ))
            }
        }
    }
}

pub fn get_extension(state: &mut State, parameters: &[Object]) -> Result<Object, RuntimeError> {
    if parameters.is_empty() {
        return Err(RuntimeError::new("No file path provided", state.last_position()));
    }

    let file_path = parameters[0].to_string();
    match std::path::Path::new(&file_path).extension() {
        Some(extension) => Ok(Object::String(Rc::new(RefCell::new(extension.to_string_lossy().to_string())))),
        None => Ok(Object::Null),
    }
}