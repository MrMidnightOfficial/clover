use std::collections::{HashMap, HashSet};
use std::fs::{read_to_string, File};

use crate::backend::dependency_solver::DependencySolver;
use crate::backend::function_state::{Scope, FunctionState};
use crate::frontend::parser::parse;
use crate::intermediate::{CompileErrorList, Position, Token, TokenValue};
use crate::intermediate::ast::{Definition, Document, IncludeDefinition, ModelDefinition, FunctionDefinition, ImplementDefinition, ApplyDefinition, Statement, Expression, IntegerExpression, FloatExpression, StringExpression, BooleanExpression, IdentifierExpression, InfixExpression, CallExpression, InstanceGetExpression, ThisExpression, PrefixExpression, IfExpression, ArrayExpression, IndexGetExpression, ForStatement, LocalDefinition};
use crate::runtime::object::{Object, make_reference};
use crate::runtime::opcode::{OpCode, Instruction};
use crate::runtime::program::{Program, Model, Function};
use crate::backend::assembly_state::AssemblyState;
use crate::runtime::runtime_info::{FileInfo, DebugInfo};
use crate::runtime::opcode::{OPERATION_ADD, OPERATION_SUB, OPERATION_MULTIPLY, OPERATION_DIVIDE, OPERATION_MOD, OPERATION_EQUAL, OPERATION_GREATER, OPERATION_LESS, OPERATION_GREATER_EQUAL, OPERATION_LESS_EQUAL, OPERATION_AND, OPERATION_OR};
use std::ops::Deref;
use std::io::{Read, Write, BufReader, BufWriter};

/// The `CompilerContext` struct represents the state of the compiler during the compilation process.
/// It holds various data structures and information needed for compiling a Clover program, such as:
///
/// - `model_definitions`: A vector of `Model` instances representing the models defined in the program.
/// - `functions`: A vector of `Function` instances representing the functions defined in the program.
/// - `constants`: A vector of `Object` instances representing the constants used in the program.
/// - `int_const_indices`: A hash map mapping integer constants to their indices in the `constants` vector.
/// - `str_const_indices`: A hash map mapping string constants to their indices in the `constants` vector.
/// - `global_dependencies`: A set of indices of global dependencies used in the program.
/// - `local_variable_count`: The number of local variables used in the program.
/// - `assembly_states`: A hash map of `AssemblyState` instances representing the assembly state for each module in the program.
/// - `local_values`: A hash map mapping local variable indices to their corresponding indices in the `assemblies` hash map.
/// - `entry_point`: The index of the entry point function in the `functions` vector.
/// - `file_info`: A `FileInfo` instance containing information about the source file.
/// - `debug_info`: A `DebugInfo` instance containing debugging information for the compiled program.
#[derive(Debug)]
pub struct CompilerContext {
    model_definitions: Vec<Model>,
    function_definitions: Vec<Function>,
    constants: Vec<Object>,

    int_const_indices: HashMap<i64, usize>,
    str_const_indices: HashMap<String, usize>,

    global_dependencies: HashSet<usize>,

    local_variable_count: usize,
    assembly_states: HashMap<String, AssemblyState>,
    local_values: HashMap<usize, usize>,

    entry_point: usize,

    file_info: FileInfo,
    debug_info: DebugInfo
}

impl CompilerContext {
    pub fn new() -> CompilerContext {
        CompilerContext {
            model_definitions: Vec::new(),
            function_definitions: Vec::new(),
            constants: Program::DEFAULT_CONSTANTS.to_vec(),

            int_const_indices: HashMap::new(),
            str_const_indices: HashMap::new(),

            global_dependencies: HashSet::new(),

            local_variable_count: 0,
            assembly_states: HashMap::new(),
            local_values: HashMap::new(),

            entry_point: 0,

            file_info: FileInfo::new(),
            debug_info: DebugInfo::new()
        }
    }

    fn add_constant_no_check(&mut self, object: Object) -> usize {
        let index = self.constants.len();
        self.constants.push(object);
        index
    }

    fn add_constant(&mut self, object: Object) -> usize {
        match &object {
            Object::Integer(value) => {
                if let Some(index) = self.int_const_indices.get(value) {
                    *index
                } else {
                    let index = self.add_constant_no_check(object.clone());
                    self.int_const_indices.insert(*value, index);
                    index
                }
            },
            Object::String(value) => {
                if let Some(index) = self.str_const_indices.get(value.borrow().deref()) {
                    *index
                } else {
                    let index = self.add_constant_no_check(object.clone());
                    self.str_const_indices.insert(value.borrow().to_string(), index);
                    index
                }
            },
            _ => self.add_constant_no_check(object)
        }
    }

    fn get_local_value(&self, local_index: usize) -> Option<Object> {
        if !self.local_values.contains_key(&local_index) {
            return None;
        };

        let &constant_index = self.local_values.get(&local_index).unwrap();

        if let Some(object) = self.constants.get(constant_index) {
            Some(object.clone())
        } else {
            None
        }
    }

    fn add_model(&mut self, model: Model) -> usize {
        let index = self.model_definitions.len();
        self.model_definitions.push(model);
        index
    }

    fn add_function(&mut self, func_state: FunctionState, name: &str, assembly_index: usize) -> usize {
        let index = self.function_definitions.len();

        let func = Function {
            parameter_count: func_state.parameter_count,
            local_variable_count: func_state.local_variable_count,
            rescue_position: func_state.rescue_position,
            is_instance: func_state.is_instance,

            instructions: func_state.instructions
        };

        self.function_definitions.push(func);
        self.debug_info.functions.push(func_state.positions);
        self.file_info.function_names.push(name.to_string());
        self.file_info.function_files.push(assembly_index);
        index
    }

    // find constant index by include definition
    fn find_constant_index_by_include(&self, assembly_name: &str, public_name: &str) -> Option<usize> {
        if let Some(assembly_state) = self.assembly_states.get(assembly_name) {
            if let Some(&index) = assembly_state.public_indices.get(public_name) {
                return Some(index);
            };
        };

        None
    }

    fn add_assembly(&mut self, assembly: AssemblyState) -> usize {
        let index = self.assembly_states.len();
        self.assembly_states.insert(assembly.filename.clone(), assembly);
        index
    }

    fn get_loaded_assemblies(&self) -> HashSet<String> {
        let mut loaded_assemblies = HashSet::new();

        for (filename, _) in self.assembly_states.iter() {
            loaded_assemblies.insert(filename.clone());
        };

        loaded_assemblies
    }

    pub fn to_program(&self) -> Program {
        Program {
            models: self.model_definitions.clone(),
            functions: self.function_definitions.clone(),
            constants: self.constants.clone(),
            global_dependencies: self.global_dependencies.iter().cloned().collect(),

            local_variable_count: self.local_variable_count,
            local_values: self.local_values.clone(),

            entry_point: self.entry_point,

            file_info: Some(self.file_info.clone()),
            debug_info: Some(self.debug_info.clone())
        }
    }
}

#[derive(Debug, Clone)]
pub struct CompilerEnv {
    pub assembly_state: AssemblyState,
    pub locals: Scope,
    pub errors: CompileErrorList
}

pub trait Storage {
    fn load_file(&self, filename: &str) -> Result<String, CompileErrorList>;

    fn get_reader(&self, filename: &str) -> Result<Box<dyn Read>, CompileErrorList>;

    fn get_writer(&self, filename: &str) -> Result<Box<dyn Write>, CompileErrorList>;
}

pub struct DefaultStorage;

impl Storage for DefaultStorage {
    fn load_file(&self, filename: &str) -> Result<String, CompileErrorList> {
        if let Ok(source) = read_to_string(filename) {
            Ok(source)
        } else {
            let mut errors = CompileErrorList::new(filename);
            errors.push_error(&Token::new(TokenValue::None, Position::none()), "Failed to open the specified source file.");
            Err(errors)
        }
    }

    fn get_reader(&self, filename: &str) -> Result<Box<dyn Read>, CompileErrorList> {
        if let Ok(file) = File::open(filename) {
            Ok(Box::new(BufReader::new(file)))
        } else {
            let mut errors = CompileErrorList::new(filename);
            errors.push_error(&Token::new(TokenValue::None, Position::none()), "Failed to open the specified source file.");
            Err(errors)
        }
    }

    fn get_writer(&self, filename: &str) -> Result<Box<dyn Write>, CompileErrorList> {
        if let Ok(file) = File::create(filename) {
            Ok(Box::new(BufWriter::new(file)))
        } else {
            let mut errors = CompileErrorList::new(filename);
            errors.push_error(&Token::new(TokenValue::None, Position::none()), "Failed to open the specified source file.");
            Err(errors)
        }
    }
}

impl DefaultStorage {
    pub fn new() -> DefaultStorage {
        DefaultStorage {}
    }
}

impl CompilerEnv {
    fn define_local(&mut self, context: &mut CompilerContext, name: &str) -> Option<usize> {
        if self.locals.contains_key(name) {
            None
        } else {
            let index = context.local_variable_count;
            self.locals.insert(name.to_string(), index);
            context.local_variable_count += 1;
            Some(index)
        }
    }

    fn define_local_by_identifier(&mut self, context: &mut CompilerContext, token: &Token) -> Option<usize> {
        if let TokenValue::Identifier(identifier) = &token.value {
            self.define_local(context, identifier)
        } else {
            self.errors.push_error(token, "variable already exists");
            None
        }
    }

    fn compile_integer_expression(&mut self, context: &mut CompilerContext, func_state: &mut FunctionState, integer_expression: &IntegerExpression) {
        if let TokenValue::Integer(value) = integer_expression.token.value {
            let index = context.add_constant(Object::Integer(value));
            func_state.emit(OpCode::PushConstant.to_instruction(index as u64), integer_expression.token.position);
        }
    }

    fn compile_float_expression(&mut self, context: &mut CompilerContext, func_state: &mut FunctionState, float_expression: &FloatExpression) {
        if let TokenValue::Float(value) = float_expression.token.value {
            let index = context.add_constant(Object::Float(value));
            func_state.emit(OpCode::PushConstant.to_instruction(index as u64), float_expression.token.position);
        }
    }

    fn compile_string_expression(&mut self, context: &mut CompilerContext, func_state: &mut FunctionState, string_expression: &StringExpression) {
        if let TokenValue::String(value) = &string_expression.token.value {
            let index = context.add_constant(Object::String(make_reference(value.clone())));
            func_state.emit(OpCode::PushConstant.to_instruction(index as u64), string_expression.token.position);
        }
    }

    fn compile_boolean_expression(&mut self, _context: &mut CompilerContext, func_state: &mut FunctionState, bool_expression: &BooleanExpression) {
        match bool_expression.token.value {
            TokenValue::True => { func_state.emit(OpCode::PushConstant.to_instruction(Program::TRUE_CONSTANT_INDEX as u64), bool_expression.token.position); },
            TokenValue::False => { func_state.emit(OpCode::PushConstant.to_instruction(Program::FALSE_CONSTANT_INDEX as u64), bool_expression.token.position); },
            _ => self.errors.push_error(&bool_expression.token, "Unexpect token")
        }
    }

    fn compile_identifier_expression(&mut self, context: &mut CompilerContext, func_state: &mut FunctionState, identifier_expression: &IdentifierExpression) {
        let identifier = identifier_expression.token.value.to_string();

        if let Some(index) = func_state.find_local(&identifier) {
            func_state.emit(OpCode::LocalGet.to_instruction(index as u64), identifier_expression.token.position);
        } else if let Some(&index) = self.locals.get(&identifier) {
            func_state.emit(OpCode::ContextGet.to_instruction(index as u64), identifier_expression.token.position);
        } else {
            let index = context.add_constant(Object::String(make_reference(identifier)));
            context.global_dependencies.insert(index);
            func_state.emit(OpCode::GlobalGet.to_instruction(index as u64), identifier_expression.token.position);
        }
    }

    fn compile_this_expression(&mut self, _context: &mut CompilerContext, func_state: &mut FunctionState, this_expression: &ThisExpression) {
        func_state.emit(OpCode::LocalGet.to_instruction(0 as u64), this_expression.token.position);
    }

    fn compile_indexed_set(&mut self, context: &mut CompilerContext, func_state: &mut FunctionState, instance: &Box<Expression>, index: &Box<Expression>, op_code: OpCode, position: Position) {
        self.compile_expression(context, func_state, instance.deref());
        self.compile_expression(context, func_state, index.deref());
        func_state.emit_opcode(op_code, position);
    }
    
    fn compile_assign_expression_left_part(&mut self, context: &mut CompilerContext, func_state: &mut FunctionState, infix_expression: &InfixExpression) {
        let left_expression = infix_expression.left.deref();

        match left_expression {
            Expression::Identifier(identifier_expression) => {
                let identifier = identifier_expression.token.value.to_string();

                if let Some(index) = func_state.find_local(&identifier) {
                    func_state.emit(OpCode::LocalSet.to_instruction(index as u64), infix_expression.infix.position);
                } else if let Some(&index) = self.locals.get(&identifier) {
                    func_state.emit(OpCode::ContextSet.to_instruction(index as u64), infix_expression.infix.position);
                } else {
                    let index = context.add_constant(Object::String(make_reference(identifier)));
                    context.global_dependencies.insert(index);
                    func_state.emit(OpCode::GlobalSet.to_instruction(index as u64), infix_expression.infix.position);
                }
            },
            Expression::InstanceGet(instance_get_expression) => {
                self.compile_indexed_set(
                    context,
                    func_state,
                    &instance_get_expression.instance,
                    &instance_get_expression.index,
                    OpCode::InstanceSet,
                    instance_get_expression.token.position
                );
            },
            Expression::IndexGet(index_get_expression) => {
                self.compile_indexed_set(
                    context,
                    func_state,
                    &index_get_expression.instance,
                    &index_get_expression.index,
                    OpCode::IndexSet,
                    index_get_expression.token.position
                );
            },
            _ => self.errors.push_error(&infix_expression.infix, "can not assign")
        }
    }

    fn compile_assign_expression(&mut self, context: &mut CompilerContext, func_state: &mut FunctionState, infix_expression: &InfixExpression) {
        self.compile_expression(context, func_state, infix_expression.right.deref());
        self.compile_assign_expression_left_part(context, func_state, infix_expression);
    }

    fn compile_infix_expression(&mut self, context: &mut CompilerContext, func_state: &mut FunctionState, infix_expression: &InfixExpression) {
        if infix_expression.infix.value == TokenValue::Assign {
            return self.compile_assign_expression(context, func_state, infix_expression);
        };

        if let Some(instruction) = get_operation_instruction_by_token(&infix_expression.infix) {
            self.compile_expression(context, func_state, infix_expression.left.deref());
            self.compile_expression(context, func_state, infix_expression.right.deref());
            func_state.emit(instruction, infix_expression.infix.position);

            if let TokenValue::NotEqual = infix_expression.infix.value {
                func_state.emit_opcode(OpCode::Not, infix_expression.infix.position);
            };

        } else {
            self.errors.push_error(&infix_expression.infix, "unknown operation");
        }

        match infix_expression.infix.value {
            TokenValue::PlusAssign | TokenValue::MinusAssign | TokenValue::StarAssign | TokenValue::SlashAssign | TokenValue::PercentAssign => { self.compile_assign_expression_left_part(context, func_state, infix_expression); },
            _ => {
                // do nothing
            }
        };
    }

    fn compile_prefix_expression(&mut self, context: &mut CompilerContext, func_state: &mut FunctionState, prefix_expression: &PrefixExpression) {
        self.compile_expression(context, func_state, prefix_expression.right.deref());

        match prefix_expression.prefix.value {
            TokenValue::Minus => { func_state.emit_opcode(OpCode::Negative, prefix_expression.prefix.position); },
            TokenValue::Not => { func_state.emit_opcode(OpCode::Not, prefix_expression.prefix.position); },
            _ => self.errors.push_error(&prefix_expression.prefix, "unknown operation")
        }
    }

    fn compile_call_expression(&mut self, context: &mut CompilerContext, func_state: &mut FunctionState, call_expression: &CallExpression) {
        // compile the function, after this the function object will on the top of stack
        self.compile_expression(context, func_state, call_expression.function.deref());
        // compile parameters
        for parameter_expression in call_expression.parameters.iter() {
            self.compile_expression(context, func_state, parameter_expression);
        };

        func_state.emit(OpCode::Call.to_instruction(call_expression.parameters.len() as u64), call_expression.token.position);
    }

    fn compile_array_expression(&mut self, context: &mut CompilerContext, func_state: &mut FunctionState, array_expression: &ArrayExpression) {
        for expression in &array_expression.values {
            self.compile_expression(context, func_state, expression);
        };
        
        func_state.emit(OpCode::Array.to_instruction(array_expression.values.len() as u64), array_expression.token.position);
    }

    fn compile_instance_get_expression(&mut self, context: &mut CompilerContext, func_state: &mut FunctionState, instance_get_expression: &InstanceGetExpression) {
        self.compile_expression(context, func_state, instance_get_expression.instance.deref());
        self.compile_expression(context, func_state, instance_get_expression.index.deref());

        func_state.emit_opcode(OpCode::InstanceGet, instance_get_expression.token.position);
    }

    fn compile_index_get_expression(&mut self, context: &mut CompilerContext, func_state: &mut FunctionState, index_get_expression: &IndexGetExpression) {
        self.compile_expression(context, func_state, index_get_expression.instance.deref());
        self.compile_expression(context, func_state, index_get_expression.index.deref());

        func_state.emit_opcode(OpCode::IndexGet, index_get_expression.token.position);
    }

    fn compile_if_expression(&mut self, context: &mut CompilerContext, func_state: &mut FunctionState, if_expression: &IfExpression) {
        self.compile_expression(context, func_state, if_expression.condition.deref());

        let true_part_instruction_index = func_state.emit_opcode_without_position(OpCode::JumpIf);

        if let Some(statements) = if_expression.false_part.as_ref() {
            func_state.enter_scope();
            for statement in statements {
                self.compile_statement(context, func_state, statement);
            }
            func_state.exit_scope();

            func_state.remove_pop_or_push_null();
        } else {
            func_state.emit(OpCode::PushConstant.to_instruction(Program::NULL_CONSTANT_INDEX as u64), func_state.get_last_position());
        };

        let jump_to_end_instruction_index = func_state.emit_opcode_without_position(OpCode::Jump);

        func_state.replace_instruction(true_part_instruction_index, OpCode::JumpIf.to_instruction(func_state.get_next_instruction_index() as u64));

        func_state.enter_scope();
        for statement in &if_expression.true_part {
            self.compile_statement(context, func_state, statement);
        }
        func_state.exit_scope();

        func_state.remove_pop_or_push_null();

        func_state.replace_instruction(jump_to_end_instruction_index,  OpCode::Jump.to_instruction(func_state.get_next_instruction_index() as u64));

    }

    fn compile_expression(&mut self, context: &mut CompilerContext, func_state: &mut FunctionState, expression: &Expression) {
        match expression {
            Expression::Integer(integer_expression) => self.compile_integer_expression(context, func_state, integer_expression),
            Expression::Float(float_expression) => self.compile_float_expression(context, func_state, float_expression),
            Expression::String(string_expression) => self.compile_string_expression(context, func_state, string_expression),
            Expression::Boolean(bool_expression) => self.compile_boolean_expression(context, func_state, bool_expression),
            Expression::Null(null_expression) => { func_state.emit(OpCode::PushConstant.to_instruction(Program::NULL_CONSTANT_INDEX as u64), null_expression.token.position); },
            Expression::Array(array_expression) => self.compile_array_expression(context, func_state, array_expression),
            Expression::Identifier(identifier_expression) => self.compile_identifier_expression(context, func_state, identifier_expression),
            Expression::Prefix(prefix_expression) => self.compile_prefix_expression(context, func_state, prefix_expression),
            Expression::Infix(infix_expression) => self.compile_infix_expression(context, func_state, infix_expression),
            Expression::Call(call_expression) => self.compile_call_expression(context, func_state, call_expression),
            Expression::InstanceGet(instance_get_expression) => self.compile_instance_get_expression(context, func_state, instance_get_expression),
            Expression::IndexGet(index_get_expression) => self.compile_index_get_expression(context, func_state, index_get_expression),
            Expression::This(this_expression) => self.compile_this_expression(context, func_state, this_expression),
            Expression::If(if_expression) => self.compile_if_expression(context, func_state, if_expression)
        }
    }

    fn compile_for_statement(&mut self, context: &mut CompilerContext, func_state: &mut FunctionState, for_statement: &ForStatement) {
        let enumerable_local_index = func_state.define_anonymous_local();
        let iterator_local_index = func_state.define_anonymous_local();

        func_state.enter_scope();
        func_state.enter_break_scope();

        // prepare the enumerable object to local
        self.compile_expression(context, func_state, &for_statement.enumerable);
        func_state.emit(OpCode::LocalSet.to_instruction(enumerable_local_index as u64), func_state.get_last_position());
        func_state.emit_opcode_without_position(OpCode::Pop);

        // prepare the iterator to local
        func_state.emit(OpCode::PushConstant.to_instruction(context.add_constant(Object::Integer(0)) as u64), func_state.get_last_position());
        func_state.emit(OpCode::LocalSet.to_instruction(iterator_local_index as u64), func_state.get_last_position());
        func_state.emit_opcode_without_position(OpCode::Pop);

        // because we just enter a new scope, so we never have a duplicate name here, so can unwrap directly
        let local_variable_index = func_state.define_local(&for_statement.identifier.value.to_string()).unwrap();

        let start_loop_position = func_state.get_next_instruction_index();

        func_state.emit(OpCode::ForNext.to_instruction(enumerable_local_index as u64), func_state.get_last_position());

        let jump_to_end_if_true_instruction_index = func_state.get_next_instruction_index();
        func_state.emit_opcode_without_position(OpCode::JumpIf);

        // set the iterator to local
        func_state.emit(OpCode::LocalSet.to_instruction(local_variable_index as u64), for_statement.identifier.position);
        func_state.emit_opcode_without_position(OpCode::Pop);

        for statement in &for_statement.statements {
            self.compile_statement(context, func_state, statement);
        };
        func_state.emit(OpCode::Iterate.to_instruction(iterator_local_index as u64), func_state.get_last_position());
        func_state.emit(OpCode::Jump.to_instruction(start_loop_position as u64), func_state.get_last_position());

        let end_position = func_state.get_next_instruction_index();

        func_state.replace_instruction(jump_to_end_if_true_instruction_index, OpCode::JumpIf.to_instruction(end_position as u64));

        func_state.exit_break_scope();
        func_state.exit_scope();

    }

    fn compile_statement(&mut self, context: &mut CompilerContext, func_state: &mut FunctionState, statement: &Statement) {
        func_state.current_depth += 1;

        match statement {
            Statement::Return(return_statement) => func_state.emit_return(return_statement.token.position),
            Statement::Expression(expression) => {
                self.compile_expression(context, func_state, expression);
                func_state.emit_opcode_without_position(OpCode::Pop);
            },
            Statement::Local(local_statement) => {
                for (i, token) in local_statement.variables.iter().enumerate() {
                    if let Some(index) = func_state.define_local(&token.value.to_string()) {
                        if let Some(expression) = local_statement.values.get(i).unwrap() {
                            self.compile_expression(context, func_state, expression);
                            func_state.emit(OpCode::LocalInit.to_instruction(index as u64), token.position);
                        }
                    } else {
                        self.errors.push_error(token, "variable already exists");
                    };
                }
            },
            Statement::Break(break_statement) => func_state.emit_break(break_statement.token.position),
            Statement::Rescue(rescue_statement) => {
                if func_state.current_depth > 1 {
                    self.errors.push_error(&rescue_statement.token, "rescue can only in the layer of function");
                } else {
                    func_state.emit_return(rescue_statement.token.position);
                    func_state.rescue_position = func_state.get_next_instruction_index();
                }
            },
            Statement::For(for_statement) => self.compile_for_statement(context, func_state, for_statement)
        }
        func_state.current_depth -= 1;
    }

    fn compile_include_definition(&mut self, context: &mut CompilerContext, include_definition: &IncludeDefinition) {
        for (i, alias) in include_definition.aliases.iter().enumerate() {
            if let Some(index) = self.define_local_by_identifier(context, alias) {
                let public_name = include_definition.public_names.get(i).unwrap();

                if let Some(constant_index) = context.find_constant_index_by_include(&include_definition.filename.value.to_string(), &public_name.value.to_string()) {
                    context.local_values.insert(index, constant_index);
                }
            }
        }
    }

    fn compile_local_definition(&mut self, context: &mut CompilerContext, local_definition: &LocalDefinition) {
        for (i, token) in local_definition.variables.iter().enumerate() {
            let local_index = self.define_local_by_identifier(context, token);

            if local_index.is_none() {
                continue;
            };

            let value = &local_definition.values[i];

            if value.is_none() {
                continue;
            };

            let constant_index = match value.clone().unwrap().value {
                TokenValue::Null => Program::NULL_CONSTANT_INDEX,
                TokenValue::True => Program::TRUE_CONSTANT_INDEX,
                TokenValue::False => Program::FALSE_CONSTANT_INDEX,
                TokenValue::Integer(integer) => context.add_constant(Object::Integer(integer)),
                TokenValue::Float(float) => context.add_constant(Object::Float(float)),
                _ => {
                    self.errors.push_error(&value.clone().unwrap(), "value in local definition can be constant only");
                    continue;
                }
            };

            context.local_values.insert(local_index.unwrap(), constant_index);
        };

    }

    // return model constant index
    fn compile_model_definition(&mut self, context: &mut CompilerContext, model_definition: &ModelDefinition) -> usize {
        let mut model = Model::new();

        for token in model_definition.properties.iter() {
            if !model.add_property(&token.value.to_string()) {
                self.errors.push_error(token, "property already exists");
            }
        }

        let model_index = context.add_model(model);
        let constant_index = context.add_constant(Object::Model(model_index));

        if let Some(local_index) = self.define_local_by_identifier(context, &model_definition.name) {
            context.local_values.insert(local_index, constant_index);
        };

        context.file_info.model_files.push(self.assembly_state.index);
        context.file_info.model_names.push(model_definition.name.value.to_string());

        constant_index
    }

    fn compile_public_model_definition(&mut self, context: &mut CompilerContext, model_definition: &ModelDefinition) {
        let constant_index = self.compile_model_definition(context, model_definition);

        self.assembly_state.public_indices.insert(model_definition.name.value.to_string(), constant_index);
    }

    fn compile_function_definition_base(&mut self, context: &mut CompilerContext, function_definition: &FunctionDefinition) -> FunctionState {
        let mut func_state = FunctionState::new();

        for parameter in function_definition.parameters.iter() {
            if TokenValue::This == parameter.value {
                func_state.is_instance = true;
            };

            if func_state.define_local(&parameter.value.to_string()).is_none() {
                self.errors.push_error(parameter, "parameter already exists");
            };
        };

        func_state.parameter_count = function_definition.parameters.len();

        for statement in function_definition.body.iter() {
            self.compile_statement(context, &mut func_state, statement);
        };

        func_state.emit_return(func_state.get_last_position());

        func_state
    }

    // return constant index
    fn compile_function_definition(&mut self, context: &mut CompilerContext, function_definition: &FunctionDefinition) -> usize {
        // define function before compile function body, so we can do recursive call
        let local_index = self.define_local_by_identifier(context, &function_definition.name);

        let func_state = self.compile_function_definition_base(context, function_definition);

        // can not have instance function here
        if func_state.is_instance {
            self.errors.push_error(&function_definition.name, "Instance functions can only be defined inside an implement block.");
            0
        } else {
            let function_index = context.add_function(func_state, &function_definition.name.value.to_string(), self.assembly_state.index);
            let constant_index = context.add_constant(Object::Function(function_index));

            if let Some(index) = local_index {
                context.local_values.insert(index, constant_index);
            };

            if &function_definition.name.value.to_string() == "main" {
                context.entry_point = function_index;
            };

            constant_index
        }
    }

    fn compile_public_function_definition(&mut self, context: &mut CompilerContext, function_definition: &FunctionDefinition) {
        let constant_index = self.compile_function_definition(context, function_definition);

        self.assembly_state.public_indices.insert(function_definition.name.value.to_string(), constant_index);
    }

    fn find_model_index_by_local_name(&mut self, context: &mut CompilerContext, token: &Token) -> Option<usize> {
        if let Some(&model_local_index) = self.locals.get(&token.value.to_string()) {
            if let Some(Object::Model(model_index)) = context.get_local_value(model_local_index) {
                return Some(model_index);
            } else {
                self.errors.push_error(token, "is not a model");
            }
        } else {
            self.errors.push_error(token, "can not find model");
        }

        None
    }

    fn compile_implement_definition(&mut self, context: &mut CompilerContext, implement_definition: &ImplementDefinition) {
        let mut functions: HashMap<String, usize> = HashMap::new();

        for function_definition in implement_definition.functions.iter() {
            let func_state = self.compile_function_definition_base(context, function_definition);
            let index = context.add_function(func_state, &function_definition.name.value.to_string(), self.assembly_state.index);

            functions.insert(function_definition.name.value.to_string(), index);
        }

        if let Some(model_index) = self.find_model_index_by_local_name(context, &implement_definition.model_name) {
            let model = context.model_definitions.get_mut(model_index).unwrap();

            for (name, index) in functions {
                model.functions.insert(name, index);
            };
        }
    }

    fn compile_apply_definition(&mut self, context: &mut CompilerContext, apply_definition: &ApplyDefinition) {
        let mut functions = HashMap::new();

        if let Some(model_index) = self.find_model_index_by_local_name(context, &apply_definition.source_model) {
            let model = context.model_definitions.get(model_index).unwrap();

            for (name, &index) in model.functions.iter() {
                functions.insert(name.clone(), index);
            };
        };

        if let Some(model_index) = self.find_model_index_by_local_name(context, &apply_definition.target_model) {
            let model = context.model_definitions.get_mut(model_index).unwrap();

            for (name, index) in functions{
                model.functions.insert(name, index);
            };
        };
    }

    fn compile_definition(&mut self, context: &mut CompilerContext, definition: &Definition) {
        match definition {
            Definition::Local(local_definition) => self.compile_local_definition(context, local_definition),
            Definition::Include(include_definition) => self.compile_include_definition(context, include_definition),
            Definition::Model(model_definition) => { self.compile_model_definition(context, model_definition); },
            Definition::PublicModel(model_definition) => self.compile_public_model_definition(context, model_definition),
            Definition::Function(function_definition) => { self.compile_function_definition(context, function_definition); },
            Definition::PublicFunction(function_definition) => self.compile_public_function_definition(context, function_definition),
            Definition::Implement(implement_definition) => self.compile_implement_definition(context, implement_definition),
            Definition::Apply(apply_definition) => self.compile_apply_definition(context, apply_definition)
        }
    }

    fn compile(&mut self, context: &mut CompilerContext, document: &Document) {
        for definition in document.definitions.iter() {
            self.compile_definition(context, definition);
        }
    }
}

pub fn compile_document(document: &Document, context: &mut CompilerContext) -> Result<(), CompileErrorList> {
    let mut env = CompilerEnv {
        assembly_state: AssemblyState::new(&document.filename),
        locals: Scope::new(),
        errors: CompileErrorList::new(&document.filename)
    };

    env.assembly_state.index = context.assembly_states.len();

    env.compile(context, document);

    context.add_assembly(env.assembly_state);
    context.file_info.filenames.push(document.filename.clone());

    if env.errors.is_empty() {
        Ok(())
    } else {
        Err(env.errors)
    }
}

pub fn compile_to(context: &mut CompilerContext, source: &str, filename: &str, file_loader: &dyn Storage) -> Result<(), CompileErrorList> {
    let mut documents: HashMap<String, Document> = HashMap::new();

    let mut dependency_solver = DependencySolver::new();

    let document = parse(&source, filename)?;

    let loaded_assemblies = context.get_loaded_assemblies();

    dependency_solver.solve(&document, &loaded_assemblies);

    documents.insert(document.filename.clone(), document);

    while let Some(dependency_filename) = dependency_solver.get_unsolved_filename() {
        let dependency_source = file_loader.load_file(&dependency_filename)?;
        let dependency_document = parse(&dependency_source, &dependency_filename)?;

        dependency_solver.solve(&dependency_document, &loaded_assemblies);
        documents.insert(dependency_filename, dependency_document);
    }

    while let Some(filename_to_compile) = dependency_solver.get_next_no_dependency_filename() {
        let document_to_compile = documents.get(&filename_to_compile).unwrap();

        compile_document(document_to_compile, context)?;

        dependency_solver.set_loaded(&filename_to_compile);
    }

    if !dependency_solver.is_empty() {
        let mut errors = CompileErrorList::new(filename);
        let cycle_filenames = dependency_solver.get_potential_cycle_filenames().join(", ");

        errors.push_error(&Token::new(TokenValue::None, Position::none()), &format!("there may have cycle reference in this files [{}]", cycle_filenames));
        return Err(errors);
    };

    Ok(())
}

pub fn compile_file(filename: &str, file_loader: &dyn Storage) -> Result<Program, CompileErrorList> {
    let source = file_loader.load_file(filename)?;

    compile(&source, filename, file_loader)
}

pub fn compile(source: &str, filename: &str, file_loader: &dyn Storage) -> Result<Program, CompileErrorList> {
    let mut context = CompilerContext::new();

    compile_to(&mut context, &source, filename, file_loader)?;

    Ok(context.to_program())
}

// helpers
fn get_operation_instruction_by_token(token: &Token) -> Option<Instruction> {
    let operand: usize = match token.value {
        TokenValue::Plus | TokenValue::PlusAssign => OPERATION_ADD,
        TokenValue::Minus | TokenValue::MinusAssign => OPERATION_SUB,
        TokenValue::Star | TokenValue::StarAssign => OPERATION_MULTIPLY,
        TokenValue::Slash | TokenValue::SlashAssign => OPERATION_DIVIDE,
        TokenValue::Percent | TokenValue::PercentAssign => OPERATION_MOD,
        TokenValue::Equal | TokenValue::NotEqual => OPERATION_EQUAL,
        TokenValue::Greater => OPERATION_GREATER,
        TokenValue::Less => OPERATION_LESS,
        TokenValue::GreaterEqual => OPERATION_GREATER_EQUAL,
        TokenValue::LessEqual => OPERATION_LESS_EQUAL,

        TokenValue::And => OPERATION_AND,
        TokenValue::Or => OPERATION_OR,

        _ => return None
    };

    Some(OpCode::Operation.to_instruction(operand as u64))
}