use clover::{State, Object, NativeModel};
use clover::debug::RuntimeError;
use clover::helper::make_reference;

use std::io::Write;

#[derive(Debug)]
pub struct IO;

impl NativeModel for IO {
    fn model_get(&self, key: &str) -> Result<Object, RuntimeError> {
        match key {
            "print" => Ok(Object::NativeFunction(print)),
            "readline" => Ok(Object::NativeFunction(readline)),
            "readfile" => Ok(Object::NativeFunction(readfile)),
            "writefile" => Ok(Object::NativeFunction(writefile)),
            "appendfile" => Ok(Object::NativeFunction(appendfile)),
            "exit" => Ok(Object::NativeFunction(exit)),
            "clear" => Ok(Object::NativeFunction(clear)),
            "system" => Ok(Object::NativeFunction(system)),
            //"sleep" => Ok(Object::NativeFunction(sleep)),
            //"getenv" => Ok(Object::NativeFunction(getenv)),
            //"setenv" => Ok(Object::NativeFunction(setenv)),
            //"deletefile" => Ok(Object::NativeFunction(deletefile)), // Delete a file
            //"mkdir" => Ok(Object::NativeFunction(mkdir)), // Create a directory
            //"rmdir" => Ok(Object::NativeFunction(rmdir)), // Remove a directory
            //"copyfile" => Ok(Object::NativeFunction(copyfile)), // Copy a file
            //"movefile" => Ok(Object::NativeFunction(movefile)), // Move a file
            //"currentdir" => Ok(Object::NativeFunction(currentdir)), // Get current directory
            //"chdir" => Ok(Object::NativeFunction(chdir)), // Change the working directory
            //"listdir" => Ok(Object::NativeFunction(listdir)), // List files in a directory
            //"time" => Ok(Object::NativeFunction(time)), // Get current time
            //"rand" => Ok(Object::NativeFunction(rand)), // Generate random number
            //"date" => Ok(Object::NativeFunction(date)), // Get current date
            //"toupper" => Ok(Object::NativeFunction(toupper)), // Convert string to uppercase
            //"tolower" => Ok(Object::NativeFunction(tolower)), // Convert string to lowercase
            //"split" => Ok(Object::NativeFunction(split)), // Split a string into parts
            "join" => Ok(Object::NativeFunction(join)), // Join parts into a string
            //"replace" => Ok(Object::NativeFunction(replace)), // Replace substring in a string
            //"length" => Ok(Object::NativeFunction(length)), // Get length of a string
            //"startswith" => Ok(Object::NativeFunction(startswith)), // Check if a string starts with another
            "endswith" => Ok(Object::NativeFunction(endswith)), // Check if a string ends with another

            //"push" => Ok(Object::NativeFunction(push)),

            _ => Ok(Object::Null)
        }
    }
}


pub fn print(_state: &mut State, parameters: &[ Object ]) -> Result<Object, RuntimeError> {
    for object in parameters {
        print!("{}", object.to_string());
    };

    println!();

    Ok(Object::Null)
}

fn readline(state: &mut State, _parameters: &[ Object ]) -> Result<Object, RuntimeError> {
    let mut line = String::new();
    if let Err(error) = std::io::stdin().read_line(&mut line) {
        Err(RuntimeError::new(error.to_string().as_str(), state.last_position()))
    } else {
        Ok(Object::String(make_reference(line)))
    }
}

pub fn readfile(state: &mut State, parameters: &[ Object ]) -> Result<Object, RuntimeError> {
    if parameters.is_empty() {
        return Err(RuntimeError::new("No file path provided", state.last_position()));
    }

    let file_path = parameters[0].to_string();
    match std::fs::read_to_string(&file_path) {
        Ok(contents) => Ok(Object::String(make_reference(contents))),
        Err(error) => Err(RuntimeError::new(
            format!("Failed to read file '{}': {}", file_path, error).as_str(),
            state.last_position(),
        )),
    }
}

pub fn writefile(state: &mut State, parameters: &[ Object ]) -> Result<Object, RuntimeError> {
    if parameters.is_empty() {
        return Err(RuntimeError::new("No file path provided", state.last_position()));
    }

    let file_path = parameters[0].to_string();

    match std::fs::write(&file_path, parameters[1].to_string()) {
        Ok(_) => Ok(Object::Null),
        Err(error) => Err(RuntimeError::new(
            format!("Failed to write to file '{}': {}", file_path, error).as_str(),
            state.last_position(),
        )),
    }
}

pub fn appendfile(state: &mut State, parameters: &[ Object ]) -> Result<Object, RuntimeError> {
    if parameters.is_empty() {
        return Err(RuntimeError::new("No file path provided", state.last_position()));
    }

    let file_path = parameters[0].to_string();

    match std::fs::OpenOptions::new()
        .append(true)
        .open(&file_path)
    {
        Ok(mut file) => {
            match file.write_all(parameters[1].to_string().as_bytes()) {
                Ok(_) => Ok(Object::Null),
                Err(error) => Err(RuntimeError::new(
                    format!("Failed to write to file '{}': {}", file_path, error).as_str(),
                    state.last_position(),
                )),
            }
        }
        Err(error) => Err(RuntimeError::new(
            format!("Failed to append to file '{}': {}", file_path, error).as_str(),
            state.last_position(),
        )),
    }
}

pub fn exit(state: &mut State, parameters: &[ Object ]) -> Result<Object, RuntimeError> {
    if parameters.is_empty() {
        return Err(RuntimeError::new("No exit code provided", state.last_position()));
    }

    match parameters[0].to_string().parse::<i32>() {
        Ok(exit_code) => std::process::exit(exit_code),
        Err(_) => Err(RuntimeError::new("Invalid exit code", state.last_position()))
    }
}

pub fn clear(_state: &mut State, _parameters: &[ Object ]) -> Result<Object, RuntimeError> {
    print!("\x1Bc");
    Ok(Object::Null)
}

pub fn system(state: &mut State, parameters: &[ Object ]) -> Result<Object, RuntimeError> {
    if parameters.is_empty() {
        return Err(RuntimeError::new("No command provided", state.last_position()));
    }

    let command = parameters[0].to_string();
    let args: Vec<String> = command.split_whitespace().map(String::from).collect();

    let output = std::process::Command::new(&args[0])
        .args(&args[1..])
        .output()
        .map_err(|error| RuntimeError::new(error.to_string().as_str(), state.last_position()))?;
    Ok(Object::String(make_reference(String::from_utf8_lossy(&output.stdout).to_string())))
}

// pub fn date(state: &mut State, _parameters: &[ Object ]) -> Result<Object, RuntimeError> {
//     let now = chrono::Local::now();
//     Ok(Object::String(make_reference(now.format("%Y-%m-%d").to_string())))
// }

// pub fn time(state: &mut State, _parameters: &[ Object ]) -> Result<Object, RuntimeError> {
//     let now = chrono::Local::now();
//     Ok(Object::String(make_reference(now.format("%H:%M:%S").to_string())))
// }

pub fn join(state: &mut State, parameters: &[ Object ]) -> Result<Object, RuntimeError> {
    if parameters.is_empty() {
        return Err(RuntimeError::new("No strings provided", state.last_position()));
    }

    let strings: Vec<String> = parameters.iter().map(|value| value.to_string()).collect();
    Ok(Object::String(make_reference(strings.join(" "))))
}

pub fn endswith(state: &mut State, parameters: &[ Object ]) -> Result<Object, RuntimeError> {
    if parameters.is_empty() {
        return Err(RuntimeError::new("No strings provided", state.last_position()));
    }

    let strings: Vec<String> = parameters.iter().map(|value| value.to_string()).collect();
    Ok(Object::Boolean(strings[0].ends_with(&strings[1])))
}
