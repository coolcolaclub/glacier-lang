use std::{mem, str};
use std::rc::{Rc, Weak};
use std::cell::RefCell;
use std::any::Any;

use crate::VmError;

pub type NativeFn = fn(Vec<Value>) -> Result<Value, VmError>;

#[derive(Clone)]
pub enum Value {
    None,
    Bool(bool),
    Integer(i64),
    Real(f64),
    Char(char),
    List(List),
    ListWeak(ListWeak),
    Bytes(Bytes),
    BytesBuffer(BytesBuffer),
    StringValue(StringValue),
    StringBuffer(StringBuffer),
    Function(Rc<Function>),
    NativeFn(NativeFn),
    Unknown(Rc<dyn Any>),
}

impl Value {
    pub fn get_type(&self) -> ValueType {
        match self {
            Value::None => ValueType::None,
            Value::Bool(_) => ValueType::Bool,
            Value::Integer(_) => ValueType::Integer,
            Value::Real(_) => ValueType::Real,
            Value::Char(_) => ValueType::Char,
            Value::List(_) => ValueType::List,
            Value::ListWeak(_) => ValueType::ListWeak,
            Value::Bytes(_) => ValueType::Bytes,
            Value::BytesBuffer(_) => ValueType::BytesBuffer,
            Value::StringValue(_) => ValueType::StringValue,
            Value::StringBuffer(_) => ValueType::StringBuffer,
            Value::Function(_) => ValueType::Function,
            Value::NativeFn(_) => ValueType::NativeFn,
            Value::Unknown(_) => ValueType::Unknown,
        }
    }
}

pub enum ValueType {
    None,
    Bool,
    Integer,
    Real,
    Char,
    List,
    ListWeak,
    Bytes,
    BytesBuffer,
    StringValue,
    StringBuffer,
    Function,
    NativeFn,
    Unknown
}

pub struct Function {
    pub module: List,
    pub bytecode: Bytes,
}

#[derive(Clone)]
pub struct List(pub Rc<RefCell<Vec<Value>>>);

impl List {
    pub fn from_vec(vec: Vec<Value>) -> List {
        List(Rc::new(RefCell::new(vec)))
    }

    pub fn resize(&self, len: usize) {
        let mut items = self.0.borrow_mut();
        items.resize_with(len, || Value::None)
    }

    pub fn get(&self, index: usize) -> Option<Value> {
        let items = self.0.borrow();
        Some(items.get(index)?.clone())
    }

    pub fn set(&self, index: usize, value: Value) -> Option<Value> {
        let mut items = self.0.borrow_mut();
        Some(mem::replace(items.get_mut(index)?, value))
    }

    pub fn get_slice(&self, a: usize, b: usize) -> Option<List> {
        let items = self.0.borrow();
        let vec = items.get(a..b)?.to_vec();
        Some(List::from_vec(vec))
    }

    pub fn set_slice(&self, src: &[Value], offset: usize) -> Option<()> {
        let mut items = self.0.borrow_mut();
        let dst = items.get_mut(offset..offset + src.len())?;
        for (di, si) in dst.iter_mut().zip(src.iter()) {
            *di = si.clone();
        }
        Some(())
    }

    pub fn push(&self, value: Value) {
        self.0.borrow_mut().push(value);
    }

    pub fn pop(&self) -> Option<Value> {
        self.0.borrow_mut().pop()
    }

    pub fn append(&self, mut t: Vec<Value>) {
        self.0.borrow_mut().append(&mut t);
    }

    pub fn downgrade(&self) -> ListWeak {
        ListWeak(Rc::downgrade(&self.0))
    }
}

#[derive(Clone)]
pub struct ListWeak(pub Weak<RefCell<Vec<Value>>>);

impl ListWeak {
    pub fn upgrade(&self) -> Option<List> {
        Some(List(self.0.upgrade()?))
    }
}

#[derive(Clone)]
pub struct Bytes(pub Rc<Vec<u8>>);

impl Bytes {
    pub fn get(&self, index: usize) -> Option<Value> {
        Some(Value::Integer(*self.0.get(index)? as i64))
    }
}

#[derive(Clone)]
pub struct BytesBuffer(pub Rc<RefCell<Vec<u8>>>);

impl BytesBuffer {
    pub fn from_vec(vec: Vec<u8>) -> BytesBuffer {
        BytesBuffer(Rc::new(RefCell::new(vec)))
    }

    pub fn resize(&self, len: usize) {
        let mut bytes = self.0.borrow_mut();
        bytes.resize(len, 0);
    }

    pub fn get(&self, index: usize) -> Option<Value> {
        let bytes = self.0.borrow();
        Some(Value::Integer(*bytes.get(index)? as i64))
    }

    pub fn set(&self, index: usize, value: u8) -> Option<Value> {
        let mut bytes = self.0.borrow_mut();
        Some(Value::Integer(mem::replace(bytes.get_mut(index)?, value) as i64))
    }

    pub fn set_slice(&self, src: &[u8], offset: usize) -> Option<()> {
        let mut bytes = self.0.borrow_mut();
        let dst = bytes.get_mut(offset..offset + src.len())?;
        dst.copy_from_slice(src);
        Some(())
    }

    pub fn copy_within(&self, src_offset: usize, offset: usize, len: usize) -> Option<()> {
        let mut bytes = self.0.borrow_mut();
        if src_offset + len > bytes.len() || offset + len > bytes.len() {
            return None;
        }
        bytes.copy_within(src_offset..src_offset + len, offset);
        Some(())
    }

    pub fn append(&self, t: &[u8]) {
        let mut bytes = self.0.borrow_mut();
        bytes.extend_from_slice(t);
    }
}

#[derive(Clone)]
pub struct StringValue(Bytes);

impl StringValue {
    pub fn from_bytes(bytes: Bytes) -> Result<StringValue, str::Utf8Error> {
        str::from_utf8(&**bytes.0)?;
        Ok(StringValue(bytes))
    }

    pub fn as_str(&self) -> &str {
        // should be safe since constructor guarantees Bytes is valid utf8
        unsafe { str::from_utf8_unchecked(&**self.0.0) }
    }

    pub fn as_bytes(&self) -> &Bytes {
        &self.0
    }

    pub fn get_char_at(&self, index: usize) -> Option<char> {
        self.as_str().get(index..)?.chars().next()
    }

    pub fn get_chars(&self) -> List {
        let vec = self.as_str().chars().map(|c| Value::Char(c)).collect();
        List::from_vec(vec)
    }
}

#[derive(Clone)]
pub struct StringBuffer(pub Rc<RefCell<String>>);

impl StringBuffer {
    pub fn from_string(string: String) -> StringBuffer {
        StringBuffer(Rc::new(RefCell::new(string)))
    }

    pub fn clear(&self) {
        self.0.borrow_mut().clear()
    }

    pub fn len(&self) -> usize {
        self.0.borrow().len()
    }

    pub fn append(&self, t: &str) {
        self.0.borrow_mut().push_str(t)
    }

    pub fn get_char_at(&self, index: usize) -> Option<char> {
        self.0.borrow().get(index..)?.chars().next()
    }

    pub fn get_chars(&self) -> List {
        let vec = self.0.borrow().chars().map(|c| Value::Char(c)).collect();
        List::from_vec(vec)
    }
}
