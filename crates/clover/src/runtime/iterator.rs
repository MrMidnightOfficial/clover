use crate::runtime::env::Env;
use crate::runtime::program::RuntimeError;
use crate::runtime::object::Object;

impl Env {
    /// Advances the iterator of the current enumerable object in the runtime env.
    ///
    /// This function is responsible for iterating over different types of enumerable objects
    /// (integers, arrays, and instances) and pushing the next value onto the stack. It also
    /// pushes a boolean value indicating whether the loop should continue or not.
    ///
    /// # Errors
    /// This function may return a `RuntimeError` if the enumerable object is not a valid type.
    pub fn for_next(&mut self, enumerable_index: usize) -> Result<(), RuntimeError> {
        let iterator_index = enumerable_index + 1;

        let enumerable = self.current_frame().locals[enumerable_index].clone();

        let iterator = if let Object::Integer(iterator) = self.current_frame().locals[iterator_index].clone() {
            iterator
        } else {
            0
        };

        let jump = match enumerable {
            Object::Integer(value) => {
                if iterator < value {
                    self.push(Object::Integer(iterator));
                    false
                } else {
                    // iterator greater than enumerable object, finish loop
                    true
                }
            },
            Object::Array(array) => {
                let index = iterator as usize;

                if index < array.borrow().len() {
                    self.push(array.borrow()[index].clone());
                    false
                } else {
                    // iterator greater than enumerable array len, finish loop
                    true
                }
            },
            Object::Instance(instance) => {
                let model_index = instance.borrow().model_index;
                let model = &self.get_program().models[model_index];
                let index = iterator as usize;

                if index < model.property_names.len() {
                    let object = Object::String(model.property_names[index].clone());
                    self.push(object);
                    false
                } else {
                    true
                }
            }
            _ => true
        };

        self.push(Object::Boolean(jump));

        Ok(())
    }

    pub fn iterate(&mut self, iterator_index: usize) {
        // Get the current value at the iterator_index and check if it's an integer
        if let Object::Integer(iterator) = self.current_frame().locals[iterator_index].clone() {
            // Increment the integer value and update the locals at the iterator_index
            self.current_frame_as_mut().locals[iterator_index] = Object::Integer(iterator + 1);
        };
    }
}