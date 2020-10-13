pub mod datamodel;
pub mod machine;

use std::rc::Rc;

use datamodel::{Function, NativeFn, Value, ValueType};

pub enum VmAction {
    None,
    Jump(i32),
    Call(Rc<Function>, Vec<Value>),
    CallNative(NativeFn, Vec<Value>),
    Return(Value),
}

pub enum VmError {
    StackEmpty,
    FrameRead(u8),
    IndexRead(i64),
    IndexWrite(i64),
    BytecodeRead(usize),
    Type(ValueType),
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
