use std::collections::{HashMap, LinkedList};
use std::error::Error;
use std::fmt::{Display, Formatter};

use crate::intermediate::Position;
use crate::runtime::runtime_info::{DebugInfo, FileInfo};
use crate::runtime::object::{Object, Reference, make_reference};
use crate::runtime::opcode::Instruction;
use crate::runtime::env::Frame;
use std::io::{Write, Read};
use byteorder::{ReadBytesExt, LittleEndian, WriteBytesExt};

use flate2::write::GzEncoder;
use flate2::read::GzDecoder;
use flate2::Compression;
//use bzip2::write::BzEncoder;

use color_print::cprintln;

#[derive(Debug, Clone)]
pub struct RuntimeError {
    pub message: String,
    pub position: Position,
    pub stack: LinkedList<Frame>
}

impl RuntimeError {
    pub fn new(message: &str, position: Position) -> RuntimeError {
        RuntimeError {
            message: message.to_string(),
            position,
            stack: LinkedList::new()
        }
    }
}

impl Display for RuntimeError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.write_fmt(format_args!("at ({}, {}) - {}", self.position.line, self.position.column, self.message))
        // TODO : add print call stack
    }
}

impl Error for RuntimeError {}

#[derive(Debug, Clone)]
pub struct Model {
    pub property_indices: HashMap<String, usize>,
    pub version: Vec<Reference<String>>,
    pub functions: HashMap<String, usize>,

    pub property_names: Vec<Reference<String>>
}

impl Model {
    pub fn new() -> Model {
        Model {
            property_indices: HashMap::new(),
            version: Vec::new(),
            functions: HashMap::new(),
            property_names: Vec::new()
        }
    }

    pub fn add_property(&mut self, property_name: &str) -> bool {
        if self.property_indices.contains_key(property_name) {
            return false;
        };

        self.property_indices.insert(property_name.to_string(), self.property_indices.len());
        self.property_names.push(make_reference(property_name.to_string()));

        true
    }

    fn serialize(&self, writer: &mut dyn Write) -> Result<(), std::io::Error>  {
        writer.write_u32::<LittleEndian>(self.property_names.len() as u32)?;
        

        for property_name_reference in &self.property_names {
            let property_name = property_name_reference.borrow();
            serialize_string(property_name.as_str(), writer)?;
        }

        writer.write_u32::<LittleEndian>(self.functions.len() as u32)?;
        for (function_name, &function_index) in &self.functions {
            serialize_string(function_name, writer)?;
            writer.write_u32::<LittleEndian>(function_index as u32)?;
        };

        Ok(())
    }

    fn deserialize(reader: &mut dyn Read) -> Result<Model, std::io::Error> {
        let mut model = Model::new();
        let property_count = reader.read_u32::<LittleEndian>()?;

        for _ in 0..property_count {
            let property_name = deserialize_string(reader)?;
            model.add_property(property_name.as_str());
        }

        let function_count = reader.read_u32::<LittleEndian>()?;
        for _ in 0..function_count {
            let function_name = deserialize_string(reader)?;
            let function_index = reader.read_u32::<LittleEndian>()? as usize;

            model.functions.insert(function_name, function_index);
        };

        Ok(model)
    }

}

#[derive(Debug, Clone)]
pub struct Function {
    pub parameter_count: usize,
    pub local_variable_count: usize,
    pub rescue_position: usize,
    pub is_instance: bool,

    pub instructions: Vec<Instruction>
}

impl Function {
    pub fn new() -> Function {
        Function {
            parameter_count: 0,
            local_variable_count: 0,
            rescue_position: 0,
            is_instance: false,

            instructions: Vec::new()
        }
    }

    fn serialize(&self, writer: &mut dyn Write) -> Result<(), std::io::Error>  {
        writer.write_u32::<LittleEndian>(self.parameter_count as u32)?;
        writer.write_u32::<LittleEndian>(self.local_variable_count as u32)?;
        writer.write_u32::<LittleEndian>(self.rescue_position as u32)?;
        writer.write_u8(if self.is_instance { 1 } else { 0 })?;

        writer.write_u32::<LittleEndian>(self.instructions.len() as u32)?;

        for instruction in &self.instructions {
            writer.write_u64::<LittleEndian>(instruction.into())?;
        };

        Ok(())
    }

    fn deserialize(reader: &mut dyn Read) -> Result<Function, std::io::Error> {
        let parameter_count = reader.read_u32::<LittleEndian>()? as usize;
        let local_variable_count = reader.read_u32::<LittleEndian>()? as usize;
        let rescue_position = reader.read_u32::<LittleEndian>()? as usize;
        let is_instance = reader.read_u8()? == 1;

        let instruction_count = reader.read_u32::<LittleEndian>()? as usize;
        let instructions: Result<Vec<Instruction>, std::io::Error> = (0..instruction_count)
            .map(|_| reader.read_u64::<LittleEndian>().map(Instruction::from))
            .collect();

        Ok(Function {
            parameter_count,
            local_variable_count,
            rescue_position,
            is_instance,
            instructions: instructions?,
        })
    }
}

#[derive(Debug)]
pub struct Program {
    pub models: Vec<Model>,
    pub functions: Vec<Function>,
    pub constants: Vec<Object>,

    // constant indices point to name of global
    pub global_dependencies: Vec<usize>,

    pub local_variable_count: usize,

    // use to init local variable, key is local index, value is constant index
    pub local_values: HashMap<usize, usize>,

    // entry_point - 1 is the function index
    pub entry_point: usize,

    pub file_info: Option<FileInfo>,
    pub debug_info: Option<DebugInfo>
}

fn serialize_string(string: &str, writer: &mut dyn Write) -> Result<(), std::io::Error> {
    writer.write_u32::<LittleEndian>(string.len() as u32)?;
    writer.write_all(string.as_bytes())?;
    Ok(())
}

fn deserialize_string(reader: &mut dyn Read) -> Result<String, std::io::Error> {
    let string_length = reader.read_u32::<LittleEndian>()? as usize;

    let mut buffer: Vec<u8> = vec![0; string_length];

    reader.read(&mut buffer)?;

    String::from_utf8(buffer).map_err(|_| {
        std::io::Error::new(std::io::ErrorKind::InvalidData, "can't convert bytes to string")
    })
}

impl Program {
    pub const NULL_CONSTANT_INDEX: usize = 0;
    pub const TRUE_CONSTANT_INDEX: usize = 1;
    pub const FALSE_CONSTANT_INDEX: usize = 2;
    pub const DEFAULT_CONSTANTS: [Object; 3] = [ Object::Null, Object::Boolean(true), Object::Boolean(false) ];

    const OBJECT_TYPE_INTEGER: u8 = 0;
    const OBJECT_TYPE_FLOAT: u8 = 1;
    const OBJECT_TYPE_STRING: u8 = 2;
    const OBJECT_TYPE_MODEL: u8 = 3;
    const OBJECT_TYPE_FUNCTION: u8 = 4;

    // PieScript
    const HEADER: u128 = 0x747069726353656950;

    pub fn serialize(&self, writer: &mut dyn Write, compress: bool) -> Result<(), std::io::Error> {
        writer.write_u128::<LittleEndian>(Program::HEADER)?;
        writer.write_u8(crate::version::MAJOR)?;
        writer.write_u8(crate::version::MINOR)?;
        writer.write_u8(crate::version::PATCH)?;
        writer.write_u8(0)?;
        // First, we will use Bzip2 compression
        //let mut bz_writer = BzEncoder::new(writer, bzip2::Compression::default());

        // Then, we will use Gzip compression on the Bzip2 compressed data
        //let mut gz_writer = GzEncoder::new(&mut bz_writer, flate2::Compression::default());

        // Optional Gzip compression
        let mut writer: Box<dyn Write> = if compress {
            Box::new(GzEncoder::new(writer, Compression::best()))
        } else {
            Box::new(writer)
        };

        //let mut writer  = &mut gz_writer;


        // models
        writer.write_u32::<LittleEndian>(self.models.len() as u32)?;
        for model in &self.models {
            model.serialize(writer.as_mut())?;
        };

        // functions
        writer.write_u32::<LittleEndian>(self.functions.len() as u32)?;
        for function in &self.functions {
            function.serialize(writer.as_mut())?;
        };

        // constants
        writer.write_u32::<LittleEndian>(self.constants.len() as u32)?;
        for i in Program::DEFAULT_CONSTANTS.len()..self.constants.len() {
            let object = &self.constants[i];

            match object {
                Object::Integer(value) => {
                    writer.write_u8(Program::OBJECT_TYPE_INTEGER)?;
                    writer.write_i64::<LittleEndian>(*value)?;
                },
                Object::Float(value) => {
                    writer.write_u8(Program::OBJECT_TYPE_FLOAT)?;
                    writer.write_f64::<LittleEndian>(*value)?;
                },
                Object::String(string) => {
                    writer.write_u8(Program::OBJECT_TYPE_STRING)?;
                    serialize_string(string.borrow().as_str(), writer.as_mut())?;
                },
                Object::Model(model_index) => {
                    writer.write_u8(Program::OBJECT_TYPE_MODEL)?;
                    writer.write_u32::<LittleEndian>(*model_index as u32)?;
                },
                Object::Function(function_index) => {
                    writer.write_u8(Program::OBJECT_TYPE_FUNCTION)?;
                    writer.write_u32::<LittleEndian>(*function_index as u32)?;
                },
                _ => {
                    // can't be here
                    return Err(std::io::Error::from_raw_os_error(0));
                }
            }
        };

        // global dependencies
        writer.write_u32::<LittleEndian>(self.global_dependencies.len() as u32)?;
        for &global_dependency in &self.global_dependencies {
            writer.write_u32::<LittleEndian>(global_dependency as u32)?;
        };

        // local count
        writer.write_u32::<LittleEndian>(self.local_variable_count as u32)?;

        // local values
        writer.write_u32::<LittleEndian>(self.local_values.len() as u32)?;
        for (&index, &value) in &self.local_values {
            writer.write_u32::<LittleEndian>(index as u32)?;
            writer.write_u32::<LittleEndian>(value as u32)?;
        };

        // entry point
        writer.write_u32::<LittleEndian>(self.entry_point as u32)?;

        Ok(())
    }

    pub fn deserialize(reader: &mut dyn Read, compressed: bool) -> Result<Program, std::io::Error> {
        if Program::HEADER != reader.read_u128::<LittleEndian>()? {
            cprintln!("<yellow>warn: header not match</>");
        };

        let version_checks = [
            (crate::version::MAJOR, "major"),
            (crate::version::MINOR, "minor"),
            (crate::version::PATCH, "patch"),
        ];

        for (expected, label) in version_checks.iter() {
            if expected != &reader.read_u8()? {
                cprintln!("<yellow>warn: {} version not match</>", label);
            }
        }

        if 0 != reader.read_u8()? {
            cprintln!("<yellow>warn: header end not match</>");
        };

        // Optional Gzip decompression
        let mut reader: Box<dyn Read> = if compressed {
            Box::new(GzDecoder::new(reader))
        } else {
            Box::new(reader)
        };

        // models
        let mut models = Vec::new();
        let model_count = reader.read_u32::<LittleEndian>()?;

        for _ in 0..model_count {
            models.push(Model::deserialize(reader.as_mut())?);
        };

        // functions
        let mut functions = Vec::new();
        let function_count = reader.read_u32::<LittleEndian>()?;

        for _ in 0..function_count {
            functions.push(Function::deserialize(reader.as_mut())?);
        };

        // constants
        let mut constants = Program::DEFAULT_CONSTANTS.to_vec();
        let constant_count = reader.read_u32::<LittleEndian>()? as usize;

        for _ in Program::DEFAULT_CONSTANTS.len()..constant_count {
            let object_type = reader.read_u8()?;

            let constant = match object_type {
                Program::OBJECT_TYPE_INTEGER => {
                    Object::Integer(reader.read_i64::<LittleEndian>()?)
                },
                Program::OBJECT_TYPE_FLOAT => {
                    Object::Float(reader.read_f64::<LittleEndian>()?)
                },
                Program::OBJECT_TYPE_STRING => {
                    Object::String(make_reference(deserialize_string(reader.as_mut())?))
                },
                Program::OBJECT_TYPE_MODEL => {
                    Object::Model(reader.read_u32::<LittleEndian>()? as usize)
                },
                Program::OBJECT_TYPE_FUNCTION => {
                    Object::Function(reader.read_u32::<LittleEndian>()? as usize)
                },
                _ => {
                    // can't be here
                    return Err(std::io::Error::from_raw_os_error(0));
                }
            };

            constants.push(constant);
        };

        // global dependencies
        let mut global_dependencies = Vec::new();
        let global_dependency_count = reader.read_u32::<LittleEndian>()?;
        for _ in 0..global_dependency_count {
            global_dependencies.push(reader.read_u32::<LittleEndian>()? as usize);
        };

        let local_variable_count = reader.read_u32::<LittleEndian>()? as usize;

        let mut local_values = HashMap::new();
        let local_value_count = reader.read_u32::<LittleEndian>()?;
        for _ in 0..local_value_count {
            let index = reader.read_u32::<LittleEndian>()? as usize;
            let value = reader.read_u32::<LittleEndian>()? as usize;
            local_values.insert(index, value);
        };

        let entry_point = reader.read_u32::<LittleEndian>()? as usize;

        Ok(Program {
            models,
            functions,
            constants,
            global_dependencies,

            local_variable_count,
            local_values,

            entry_point,

            file_info: None,
            debug_info: None
        })
    }

}