use crate::{VmAction, VmError};

/*
TODO this is where I stopped. still deciding how to structure the code for
parsing and running op-codes, while also sharing as much info with the code
for declaring op-codes and serializing to a buffer
*/

pub fn parse_and_run(frame: &mut CallFrame) -> Result<VmAction, VmError> {
    let cursor = frame.get_cursor();
    let bytecode = frame.get_bytecode();
    let mut data = frame.get_data_mut();
    let first_byte = bytecode.get(cursor).ok_or_else(|| VmError::BytecodeRef(cursor))?;
    let result = match first_byte {
        1 => Ok(VmAction::None),
        2..=6 => {
            //
        },
    }
}

pub enum Operation {
    None,
    // int and real math
    Add,
    Sub,
    Mul,
    Div,
    Rem,
    Neg,
    // int
    Shl,
    Shr,
    And,
    Or,
    Xor,
    Not,
    // real
    IntToReal,
    Floor,
    Ceil,
    Trunc,
    Round,
    Cmp,
    // call and jump
    Call,
    Return,
    Jump(i16),
    JumpZero(i16),
    JumpNeg(i16),
    // literal
    LiteralNone,
    LiteralTrue,
    LiteralFalse,
    LiteralInteger(i64),
    LiteralReal(f64),
    // frame
    FrameLocalLoad(u8),
    FrameLocalStore(u8),
    FrameLocalSwap(u8),
    FrameStackCopy,
    FrameStackPop,
    // list
    ListCreate,
    ListPush,
    ListPop,
    ListDowngrade,
    ListUpgrade,
    // bytes
    BytesBufferCreate,
    // string
    StringBufferCreate,
    StringGetCharAt,
    StringGetChars,
    // seq
    SeqGet,
    SeqSet,
    SeqGetSlice,
    SeqSetSlice,
    SeqAppend,
    SeqLen,
    SeqResize,
}
