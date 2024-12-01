use clover::{Env, Object, NativeModel, Reference, NativeModelInstance};
use clover::debug::{Position, RuntimeError};
use clover::helper::make_reference;
use mlua::Lua;
use crate::helper::expect_parameter_count;

#[derive(Debug)]
pub struct LuaLib;

impl NativeModel for LuaLib {

    fn call(&mut self, _state: &mut Env, _parameters: &[Object]) -> Result<Object, RuntimeError> {
        let lua = Lua::new();
        Ok(Object::NativeInstance(make_reference(LuaInstance { lua })))
    }

    fn model_get(&self, key: &str) -> Result<Object, RuntimeError> {
        match key {
            "new" => Ok(Object::NativeFunction(new)),
            _ => Ok(Object::Null)
        }
    }
}

pub struct LuaInstance {
    pub lua: Lua
}

impl NativeModelInstance for LuaInstance {
    fn index_get(&self, _this: Reference<dyn NativeModelInstance>, _index: &Object) -> Result<Object, RuntimeError> {
        Ok(Object::Null)
    }

    fn index_set(&mut self, _this: Reference<dyn NativeModelInstance>, _index: &Object, _value: Object) -> Result<(), RuntimeError> {
        Ok(())
    }

    fn instance_get(&self, this: Reference<dyn NativeModelInstance>, key: &str) -> Result<Object, RuntimeError> {
        match key {
            "run_string" | "run_file" | "get_var" => Ok(Object::InstanceNativeFunction(this, key.to_string())),
            _ => Err(RuntimeError::new("does not exist", Position::none()))
        }
    }

    fn instance_set(&mut self, _this: Reference<dyn NativeModelInstance>, _key: &str, _value: Object) -> Result<(), RuntimeError> {
        Ok(())
    }

    fn call(&mut self, _this: Reference<dyn NativeModelInstance>, env: &mut Env, key: &str, parameters: &[Object]) -> Result<Object, RuntimeError> {
        match key {
            "run_string" => self.run_string(env, parameters),
            "run_file" => self.run_file(env, parameters),
            "get_var" => self.get_var(env, parameters),
            _ =>  Err(RuntimeError::new("does not exist", Position::none()))
        }
    }
}

impl LuaInstance {
    pub fn run_string(&mut self, _env: &mut Env, parameters: &[Object]) -> Result<Object, RuntimeError> {
        expect_parameter_count(_env, parameters, 1)?;
        let code = parameters[0].to_string().trim().to_string();
        self.lua.load(&code).exec().map_err(|e| RuntimeError::new(&e.to_string(), Position::none()))?;
        Ok(Object::Null)
    }

    pub fn run_file(&mut self, _env: &mut Env, parameters: &[Object]) -> Result<Object, RuntimeError> {
        expect_parameter_count(_env, parameters, 1)?;
        let path = parameters[0].to_string().trim().to_string();
        if !path.ends_with(".lua") {
            return Err(RuntimeError::new("file must be a lua file and end with .lua", Position::none()));
        }
        self.lua.load(&format!("require('{}')", path)).exec().map_err(|e| RuntimeError::new(&e.to_string(), Position::none()))?;
        Ok(Object::Null)
    }

    pub fn get_var(&mut self, _env: &mut Env, parameters: &[Object]) -> Result<Object, RuntimeError> {
        expect_parameter_count(_env, parameters, 1)?;
        let name = parameters[0].to_string().trim().to_string();
        let name_trimmed = name.trim().to_string();
        let globals = self.lua.globals();
        // Check if the variable exists
        match globals.get::<mlua::Value>(self.lua.create_string(name).unwrap()) {
            Ok(value) => {
                match value {
                    mlua::Value::Number(x) => { Ok(Object::Float(x)) },
                    mlua::Value::Integer(x) => { Ok(Object::Integer(x)) },
                    mlua::Value::String(x) => Ok(Object::String(make_reference(x.to_str().unwrap().to_string()))),
                    mlua::Value::Boolean(x) => Ok(Object::Boolean(x)),
                    mlua::Value::Nil => {
                        println!("Variable '{}' does not exist.", &name_trimmed);
                        Ok(Object::Null)
                    },
                    _ => return Err(RuntimeError::new("Failed to get variable", Position::none())),
                }
            },
            Err(_) => return Err(RuntimeError::new("Failed to get variable", Position::none())),
        }
    }
}

fn new(_env: &mut Env, _parameters: &[Object]) -> Result<Object, RuntimeError> {
    let lua = Lua::new();
    Ok(Object::NativeInstance(make_reference(LuaInstance { lua })))
}
