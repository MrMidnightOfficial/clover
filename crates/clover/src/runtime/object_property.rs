use crate::runtime::object::{Object, Reference, make_reference};
use crate::runtime::env::Env;
use crate::runtime::program::RuntimeError;
use std::ops::Deref;

pub fn instance_get_integer(env: &mut Env, value: i64, key: &str) -> Result<(), RuntimeError> {

    let object = match key {
        "string" => Object::String(make_reference(value.to_string())),
        "integer" => Object::Integer(value),
        "float" => Object::Float(value as f64),

        _ => { return Err(RuntimeError::new("unknown property", env.last_position())); }
    };

    env.push(object);

    Ok(())
}

pub fn instance_get_float(env: &mut Env, value: f64, key: &str) -> Result<(), RuntimeError> {
    let object = match key {
        "string" => Object::String(make_reference(value.to_string())),
        "integer" => Object::Integer(value as i64),
        "float" => Object::Float(value),

        // Handle unknown property key
        _ => { return Err(RuntimeError::new("unknown property", env.last_position())); }
    };

    // Push the created object onto the env's stack
    env.push(object);

    Ok(())
}

pub fn instance_get_string(env: &mut Env, value: Reference<String>, key: &str) -> Result<(), RuntimeError> {
    let object = match key {
        "string" => Object::String(value),
        "integer" => {
            if let Ok(integer) = value.borrow().deref().parse::<i64>() {
                Object::Integer(integer)
            } else {
                Object::Null
            }
        },
        "float" => {
            if let Ok(float) = value.borrow().deref().parse::<f64>() {
                Object::Float(float)
            } else {
                Object::Null
            }
        },
        "length" => Object::Integer(value.borrow().len() as i64),
        _ => { return Err(RuntimeError::new("unknown property", env.last_position())); }
    };

    env.push(object);

    Ok(())
}

pub fn instance_get_array(env: &mut Env, array: Reference<Vec<Object>>, key: &str) -> Result<(), RuntimeError> {
    match key {
        "length" => {
            env.push(Object::Integer(array.borrow().len() as i64));
            Ok(())
        },
        _ => Err(RuntimeError::new("unknown property", env.last_position()))
    }
}
