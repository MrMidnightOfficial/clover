use clover::{Env, Object, NativeModel};
use clover::debug::RuntimeError;

#[derive(Debug)]
pub struct Array;

impl NativeModel for Array {
    fn model_get(&self, key: &str) -> Result<Object, RuntimeError> {
        match key {
            "push" => Ok(Object::NativeFunction(push)),
            "pop" => Ok(Object::NativeFunction(pop)),
            _ => Ok(Object::Null)
        }
    }
}


pub fn push(env: &mut Env, parameters: &[ Object ]) -> Result<Object, RuntimeError> {
    if parameters.is_empty() {
        return Err(RuntimeError::new("No value provided", env.last_position()));
    }

    // Get the array from the first parameter
    let array = match &parameters[0] {
        Object::Array(array) => array.clone(),
        _ => return Err(RuntimeError::new("First parameter must be an array", env.last_position()))
    };

    // Append the remaining parameters to the array
    for value in &parameters[1..] {
        array.borrow_mut().push(value.clone());
    }

    Ok(Object::Array(array))
}


pub fn pop(env: &mut Env, parameters: &[ Object ]) -> Result<Object, RuntimeError> {
    if parameters.is_empty() {
        return Err(RuntimeError::new("No value provided", env.last_position()));
    }
    let array = parameters[0].clone();

    if let Object::Array(array) = array {
        Ok(array.borrow_mut().pop().unwrap_or(Object::Null))
    } else {
        Err(RuntimeError::new("First parameter must be an array", env.last_position()))
    }
}
