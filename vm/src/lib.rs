pub mod datamodel;
pub mod machine;
pub mod op;

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
    DivByZero,
    FrameRead(u8),
    IndexRead(i64),
    IndexWrite(i64),
    SliceRead(i64, i64),
    BytecodeRead(usize),
    Type(ValueType, u8),
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
