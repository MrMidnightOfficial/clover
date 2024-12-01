use clover::{Env, Object, NativeModel, Reference, NativeModelInstance};
use clover::debug::{Position, RuntimeError};
use mlua::prelude::*;
use std::rc::Rc;
use std::cell::RefCell;
use std::collections::HashMap;
use clover::helper::make_reference;
use mlua::Lua;

#[derive(Debug)]
pub struct LuaLib;

impl NativeModel for LuaLib {

    fn call(&mut self, _state: &mut Env, _parameters: &[Object]) -> Result<Object, RuntimeError> {
        let lua = Lua::new();
        Ok(Object::NativeInstance(make_reference(luaInstance { lua })))
    }

    fn model_get(&self, key: &str) -> Result<Object, RuntimeError> {
        match key {
            "new" => Ok(Object::NativeFunction(new)),
            _ => Ok(Object::Null)
        }
    }
}

//pub struct luaInstance(HashMap<String, Object>);
pub struct luaInstance {
    pub lua: Lua
}

impl NativeModelInstance for luaInstance {
    fn index_get(&self, _this: Reference<dyn NativeModelInstance>, index: &Object) -> Result<Object, RuntimeError> {
        Ok(Object::Null)
    }

    fn index_set(&mut self, _this: Reference<dyn NativeModelInstance>, index: &Object, value: Object) -> Result<(), RuntimeError> {
        Ok(())
    }

    fn instance_get(&self, this: Reference<dyn NativeModelInstance>, key: &str) -> Result<Object, RuntimeError> {
        match key {
            "run" | "run_file" => Ok(Object::InstanceNativeFunction(this, key.to_string())),
            _ => Err(RuntimeError::new("index does not exists", Position::none()))
        }
    }

    fn instance_set(&mut self, this: Reference<dyn NativeModelInstance>, key: &str, value: Object) -> Result<(), RuntimeError> {
        Ok(())
    }

    fn call(&mut self, _this: Reference<dyn NativeModelInstance>, env: &mut Env, key: &str, parameters: &[Object]) -> Result<Object, RuntimeError> {
        match key {
            "run" => self.run(env, parameters),
            "run_file" => self.run_file(env, parameters),
            _ =>  Err(RuntimeError::new("index does not exists", Position::none()))
        }
    }
}

impl luaInstance {
    pub fn run(&mut self, _env: &mut Env, parameters: &[Object]) -> Result<Object, RuntimeError> {
        let code = parameters[0].to_string().trim().to_string();
        self.lua.load(&code).exec().map_err(|e| RuntimeError::new(&e.to_string(), Position::none()))?;
        Ok(Object::Null)
    }

    pub fn run_file(&mut self, _env: &mut Env, parameters: &[Object]) -> Result<Object, RuntimeError> {
        let path = parameters[0].to_string().trim().to_string();
        if !path.ends_with(".lua") {
            return Err(RuntimeError::new("file must be a lua file", Position::none()));
        }
        self.lua.load(&format!("require('{}')", path)).exec().map_err(|e| RuntimeError::new(&e.to_string(), Position::none()))?;
        Ok(Object::Null)
    }

}

fn new(_env: &mut Env, _parameters: &[Object]) -> Result<Object, RuntimeError> {
    let lua = Lua::new();
    Ok(Object::NativeInstance(make_reference(luaInstance { lua })))
}
