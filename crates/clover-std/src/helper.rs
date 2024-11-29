use clover::{Env, Object};
use clover::debug::RuntimeError;

pub fn expect_parameter_count(env: &Env, parameters: &[ Object ], count: usize) -> Result<(), RuntimeError> {
    if parameters.len() != count {
        return Err(RuntimeError::new(&format!("except {} parameters, got {}", count, parameters.len()), env.last_position()));
    };

    Ok(())
}

pub fn expect_float(env: &Env, object: &Object) -> Result<f64, RuntimeError> {
    match object {
        Object::Float(value) => Ok(*value),
        _ => Err(RuntimeError::new("can accept Float only", env.last_position()))
    }
}