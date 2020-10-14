use std::mem;

use crate::datamodel::{Bytes, Function, Value};
use crate::VmError;

pub struct CallStack {
    frames: Vec<CallFrame>,
}

pub struct CallFrame {
    stack: Vec<Value>,
    local: Vec<Value>,
    cursor: usize,
    bytecode: Bytes,
}

impl CallFrame {
    pub fn new(f: &Function) -> CallFrame {
        let local = vec![Value::List(f.module.clone())];
        CallFrame {
            stack: vec![],
            local,
            cursor: 0,
            bytecode: f.bytecode.clone(),
        }
    }

    pub fn get_cursor(&self) -> usize {
        self.cursor
    }

    pub fn set_cursor(&mut self, cursor: usize) {
        self.cursor = cursor
    }

    pub fn get_bytecode(&self) -> &[u8] {
        &self.bytecode.0
    }

    pub fn load(&self, index: u8) -> Result<&Value, VmError> {
        self.local.get(index as usize).ok_or_else(|| VmError::FrameRead(index))
    }

    fn get_mut_or_resize(&mut self, index: u8) -> &mut Value {
        let index = index as usize;
        if index >= self.local.len() {
            self.local.resize_with(index + 1, || Value::None);
        }
        &mut self.local[index]
    }

    pub fn store(&mut self, index: u8, val: Value) {
        let out = self.get_mut_or_resize(index);
        *out = val;
    }

    pub fn swap(&mut self, index: u8, val: &mut Value) {
        let out = self.get_mut_or_resize(index);
        mem::swap(out, val);
    }

    pub fn push(&mut self, val: Value) {
        self.stack.push(val);
    }

    pub fn pop(&mut self) -> Result<Value, VmError> {
        self.stack.pop().ok_or(VmError::StackEmpty)
    }
}
