use std::convert::TryInto;
use std::cmp::Ordering;

use crate::{
    VmAction, VmError,
    datamodel::{BytesBuffer, List, StringBuffer, Value},
    machine::{CallFrame},
};
/*
10-13
still deciding how to structure the code for
parsing and running op-codes, while also sharing as much info with the code
for declaring op-codes and serializing to a buffer

10-14
I decided to use named consts to assign number code to each operation. for serializing to
a buffer, we'll manually write a match statement to write the correct number code based on
the named constants.
*/

macro_rules! type_err {
    ($t:expr, $pos:expr) => {
        return Err(VmError::Type($t.get_type(), $pos))
    };
}

macro_rules! bytecode_take {
    ($frame:expr, $cursor:expr, $n:expr) => {
        {
            let t = $frame.get_bytecode()
            .get($cursor..$cursor+$n)
            .ok_or_else(|| VmError::BytecodeRead($cursor))?;
            $cursor += $n;
            t
        }
    };
}

macro_rules! bytecode_next {
    ($frame:expr, $cursor:expr) => {
        {
            let t = $frame.get_bytecode()
            .get($cursor)
            .ok_or_else(|| VmError::BytecodeRead($cursor))?;
            $cursor += 1;
            t
        }
    };
}

macro_rules! math_op {
    ($frame:expr, $closure:expr) => {
        {
            let rhs = $frame.pop()?;
            let lhs = $frame.pop()?;
            let out = match lhs {
                Value::Integer(lhs) => match rhs {
                    Value::Integer(rhs) => Value::Integer($closure(lhs, rhs)),
                    _ => type_err!(rhs, 0),
                },
                Value::Real(lhs) => match rhs {
                    Value::Real(rhs) => Value::Real($closure(lhs, rhs)),
                    _ => type_err!(rhs, 0),
                }
                _ => type_err!(lhs, 1),
            };
            $frame.push(out);
            Ok(VmAction::None)
        }
    };
}

macro_rules! int_op {
    ($frame:expr, $closure:expr) => {
        {
            let rhs = $frame.pop()?;
            let lhs = $frame.pop()?;
            let out = match lhs {
                Value::Integer(lhs) => match rhs {
                    Value::Integer(rhs) => Value::Integer($closure(lhs, rhs)),
                    _ => type_err!(rhs, 0),
                },
                _ => type_err!(lhs, 1),
            };
            $frame.push(out);
            Ok(VmAction::None)
        }
    };
}

pub fn parse_and_run(frame: &mut CallFrame) -> Result<VmAction, VmError> {
    let mut cursor = frame.get_cursor();
    let op_code = *frame.get_bytecode().get(cursor).ok_or_else(|| VmError::BytecodeRead(cursor))?;
    cursor += 1;
    let result = match op_code {
        NONE => Ok(VmAction::None),
        ADD => math_op!(frame, |lhs, rhs| lhs + rhs),
        SUB => math_op!(frame, |lhs, rhs| lhs - rhs),
        MUL => math_op!(frame, |lhs, rhs| lhs * rhs),
        DIV => {
            let rhs = frame.pop()?;
            let lhs = frame.pop()?;
            let out = match lhs {
                Value::Integer(lhs) => match rhs {
                    Value::Integer(rhs) => Value::Integer(
                        lhs.checked_div(rhs).ok_or_else(|| VmError::DivByZero)?),
                    _ => type_err!(rhs, 0),
                },
                Value::Real(lhs) => match rhs {
                    Value::Real(rhs) => Value::Real(lhs / rhs),
                    _ => type_err!(rhs, 0),
                }
                _ => type_err!(lhs, 1),
            };
            frame.push(out);
            Ok(VmAction::None)
        },
        REM => {
            let rhs = frame.pop()?;
            let lhs = frame.pop()?;
            let out = match lhs {
                Value::Integer(lhs) => match rhs {
                    Value::Integer(rhs) => Value::Integer(
                        lhs.checked_rem(rhs).ok_or_else(|| VmError::DivByZero)?),
                    _ => type_err!(rhs, 0),
                },
                Value::Real(lhs) => match rhs {
                    Value::Real(rhs) => Value::Real(lhs % rhs),
                    _ => type_err!(rhs, 0),
                }
                _ => type_err!(lhs, 1),
            };
            frame.push(out);
            Ok(VmAction::None)
        },
        NEG => {
            let t = frame.pop()?;
            let out = match t {
                Value::Integer(t) => Value::Integer(-t),
                Value::Real(t) => Value::Real(-t),
                _ => type_err!(t, 0),
            };
            frame.push(out);
            Ok(VmAction::None)
        },
        SHL => int_op!(frame, |lhs, rhs| lhs << rhs),
        SHR => int_op!(frame, |lhs, rhs| lhs >> rhs),
        AND => int_op!(frame, |lhs, rhs| lhs & rhs),
        OR  => int_op!(frame, |lhs, rhs| lhs | rhs),
        XOR => int_op!(frame, |lhs, rhs| lhs ^ rhs),
        NOT => {
            let t = frame.pop()?;
            let out = match t {
                Value::Integer(t) => Value::Integer(!t),
                _ => type_err!(t, 0),
            };
            frame.push(out);
            Ok(VmAction::None)
        },
        INT_TO_REAL => {
            let t = frame.pop()?;
            let out = match t {
                Value::Integer(t) => Value::Real(t as f64),
                Value::Real(t) => Value::Real(t),
                _ => type_err!(t, 0),
            };
            frame.push(out);
            Ok(VmAction::None)
        },
        REAL_TO_INT => {
            let t = frame.pop()?;
            let out = match t {
                Value::Integer(t) => Value::Integer(t),
                Value::Real(t) => Value::Integer(t as i64),
                _ => type_err!(t, 0),
            };
            frame.push(out);
            Ok(VmAction::None)
        },
        CMP => {
            let rhs = frame.pop()?;
            let lhs = frame.pop()?;
            let result = match lhs.cmp(&rhs) {
                Some(order) => Value::Integer(match order {
                    Ordering::Less => -1,
                    Ordering::Equal => 0,
                    Ordering::Greater => 1,
                }),
                None => Value::None,
            };
            frame.push(result);
            Ok(VmAction::None)
        },
        CALL => {
            let num_args = *bytecode_next!(frame, cursor) as usize;
            let fn_target = frame.pop()?;
            let mut args = Vec::new();
            for _ in 0..num_args {
                args.push(frame.pop()?);
            }
            match fn_target {
                Value::Function(f) => Ok(VmAction::Call(f, args)),
                Value::NativeFn(f) => Ok(VmAction::CallNative(f, args)),
                _ => type_err!(fn_target, 0),
            }
        },
        RETURN => Ok(VmAction::Return(frame.pop()?)),
        JUMP => {
            let dst = bytecode_take!(frame, cursor, 2);
            let dst = i16::from_be_bytes([dst[0], dst[1]]);
            Ok(VmAction::Jump(dst))
        },
        JUMP_ZERO => {
            let dst = bytecode_take!(frame, cursor, 2);
            let dst = i16::from_be_bytes([dst[0], dst[1]]);
            let check = match frame.pop()? {
                Value::None => true,
                Value::Bool(t) => !t,
                Value::Integer(t) => t == 0,
                Value::Real(t) => t == 0.0,
                e @ _ => type_err!(e, 0),
            };
            if check {
                Ok(VmAction::Jump(dst))
            } else {
                Ok(VmAction::None)
            }
        },
        JUMP_NEG => {
            let dst = bytecode_take!(frame, cursor, 2);
            let dst = i16::from_be_bytes([dst[0], dst[1]]);
            let check = match frame.pop()? {
                Value::Integer(t) => t < 0,
                Value::Real(t) => t < 0.0,
                e @ _ => type_err!(e, 0),
            };
            if check {
                Ok(VmAction::Jump(dst))
            } else {
                Ok(VmAction::None)
            }
        },
        LIT_NONE => {
            frame.push(Value::None);
            Ok(VmAction::None)
        },
        LIT_TRUE => {
            frame.push(Value::Bool(true));
            Ok(VmAction::None)
        },
        LIT_FALSE => {
            frame.push(Value::Bool(false));
            Ok(VmAction::None)
        },
        LIT_INT => {
            let b = bytecode_take!(frame, cursor, 8);
            let i = i64::from_be_bytes(b.try_into().unwrap());
            frame.push(Value::Integer(i));
            Ok(VmAction::None)
        },
        LIT_REAL => {
            let b = bytecode_take!(frame, cursor, 8);
            let r = f64::from_be_bytes(b.try_into().unwrap());
            frame.push(Value::Real(r));
            Ok(VmAction::None)
        },
        FRM_LOAD => {
            let i = *bytecode_next!(frame, cursor);
            frame.push(frame.load(i)?.clone());
            Ok(VmAction::None)
        },
        FRM_STORE => {
            let i = *bytecode_next!(frame, cursor);
            let t = frame.pop()?;
            frame.store(i, t);
            Ok(VmAction::None)
        },
        FRM_SWAP => {
            let i = *bytecode_next!(frame, cursor);
            let mut t = frame.pop()?;
            frame.swap(i, &mut t);
            frame.push(t);
            Ok(VmAction::None)
        },
        FRM_COPY => {
            let t = frame.pop()?;
            frame.push(t.clone());
            frame.push(t);
            Ok(VmAction::None)
        },
        FRM_POP => {
            frame.pop()?;
            Ok(VmAction::None)
        },
        LIST_CREATE => {
            let t = Value::List(List::from_vec(vec![]));
            frame.push(t);
            Ok(VmAction::None)
        },
        LIST_PUSH => {
            let ele = frame.pop()?;
            let list = match frame.pop()? {
                Value::List(t) => t,
                e @ _ => type_err!(e, 1),
            };
            list.push(ele);
            Ok(VmAction::None)
        },
        LIST_POP => {
            let list = match frame.pop()? {
                Value::List(t) => t,
                e @ _ => type_err!(e, 0),
            };
            let ele = list.pop().ok_or_else(|| VmError::IndexRead(0))?;
            frame.push(ele);
            Ok(VmAction::None)
        },
        LIST_DOWNGRADE => {
            let list = match frame.pop()? {
                Value::List(t) => t,
                e @ _ => type_err!(e, 0),
            };
            let weak = Value::ListWeak(list.downgrade());
            frame.push(weak);
            Ok(VmAction::None)
        },
        LIST_UPGRADE => {
            let weak = match frame.pop()? {
                Value::ListWeak(t) => t,
                e @ _ => type_err!(e, 0),
            };
            let out = match weak.upgrade() {
                Some(list) => Value::List(list),
                None => Value::None,
            };
            frame.push(out);
            Ok(VmAction::None)
        },
        BYTES_CREATE => {
            let t = Value::BytesBuffer(BytesBuffer::from_vec(vec![]));
            frame.push(t);
            Ok(VmAction::None)
        },
        STR_CREATE => {
            let t = Value::StringBuffer(StringBuffer::from_string(String::new()));
            frame.push(t);
            Ok(VmAction::None)
        },
        STR_CHAR_AT => {
            let i = match frame.pop()? {
                Value::Integer(i) => i as usize,
                e @ _ => type_err!(e, 0),
            };
            let c = match frame.pop()? {
                Value::StringValue(s) => s.get_char_at(i),
                Value::StringBuffer(s) => s.get_char_at(i),
                e @ _ => type_err!(e, 1),
            };
            let v = match c {
                Some(c) => Value::Char(c),
                None => Value::None,
            };
            frame.push(v);
            Ok(VmAction::None)
        },
        STR_CHARS => {
            let chars = match frame.pop()? {
                Value::StringValue(s) => s.get_chars(),
                Value::StringBuffer(s) => s.get_chars(),
                e @ _ => type_err!(e, 0),
            };
            frame.push(Value::List(chars));
            Ok(VmAction::None)
        },
        SEQ_GET => {
            let i = match frame.pop()? {
                Value::Integer(i) => i,
                e @ _ => type_err!(e, 0),
            };
            let out = match frame.pop()? {
                Value::List(l) => l.get(i as usize),
                Value::Bytes(b) => b.get(i as usize),
                Value::BytesBuffer(b) => b.get(i as usize),
                e @ _ => type_err!(e, 1),
            }.ok_or_else(|| VmError::IndexRead(i))?;
            frame.push(out);
            Ok(VmAction::None)
        },
        SEQ_SET => {
            let v = frame.pop()?;
            let i = match frame.pop()? {
                Value::Integer(i) => i,
                e @ _ => type_err!(e, 1),
            };
            match frame.pop()? {
                Value::List(l) => l.set(i as usize, v),
                Value::BytesBuffer(b) => {
                    let v = match v {
                        Value::Integer(v) => v as u8,
                        e @ _ => type_err!(e, 0),
                    };
                    b.set(i as usize, v)
                },
                e @ _ => type_err!(e, 2),
            }.ok_or_else(|| VmError::IndexWrite(i))?;
            Ok(VmAction::None)
        },
        SEQ_GET_SLICE => {
            let end = match frame.pop()? {
                Value::Integer(i) => i,
                e @ _ => type_err!(e, 0),
            };
            let start = match frame.pop()? {
                Value::Integer(i) => i,
                e @ _ => type_err!(e, 1),
            };
            let out = match frame.pop()? {
                Value::List(l) => l.get_slice(start as usize, end as usize).map(
                    |l| Value::List(l)),
                Value::Bytes(b) => b.get_slice(start as usize, end as usize).map(
                    |b| Value::BytesBuffer(b)),
                Value::BytesBuffer(b) => b.get_slice(start as usize, end as usize).map(
                    |b| Value::BytesBuffer(b)),
                e @ _ => type_err!(e, 2),
            }.ok_or_else(|| VmError::SliceRead(start, end))?;
            frame.push(out);
            Ok(VmAction::None)
        },
        SEQ_SET_SLICE => {
            todo!()
        },
        SEQ_APPEND => {
            todo!()
        },
        SEQ_LEN => {
            let len = match frame.pop()? {
                Value::List(l) => l.len(),
                Value::Bytes(b) => b.len(),
                Value::BytesBuffer(b) => b.len(),
                Value::StringValue(s) => s.as_str().len(),
                Value::StringBuffer(s) => s.len(),
                e @ _ => type_err!(e, 0),
            };
            frame.push(Value::Integer(len as i64));
            Ok(VmAction::None)
        },
        SEQ_RESIZE => {
            let len = match frame.pop()? {
                Value::Integer(i) => i,
                e @ _ => type_err!(e, 0),
            } as usize;
            match frame.pop()? {
                Value::List(l) => l.resize(len),
                Value::BytesBuffer(b) => b.resize(len),
                e @ _ => type_err!(e, 1),
            }
            Ok(VmAction::None)
        },
        _ => return Err(VmError::BytecodeRead(cursor))
    };
    frame.set_cursor(cursor);
    return result;
}

pub const NONE: u8 = 1;
// math
pub const ADD: u8 = 2;
pub const SUB: u8 = 3;
pub const MUL: u8 = 4;
pub const DIV: u8 = 5;
pub const REM: u8 = 6;
pub const NEG: u8 = 7;
// int
pub const SHL: u8 = 8;
pub const SHR: u8 = 9;
pub const AND: u8 = 10;
pub const OR: u8 = 11;
pub const XOR: u8 = 12;
pub const NOT: u8 = 13;
// real
pub const INT_TO_REAL: u8 = 14;
pub const REAL_TO_INT: u8 = 15;
pub const CMP: u8 = 19;
// call and jump
pub const CALL: u8 = 20;
pub const RETURN: u8 = 21;
pub const JUMP: u8 = 22;
pub const JUMP_ZERO: u8 = 23;
pub const JUMP_NEG: u8 = 24;
// literal
pub const LIT_NONE: u8 = 30;
pub const LIT_TRUE: u8 = 31;
pub const LIT_FALSE: u8 = 32;
pub const LIT_INT: u8 = 33;
pub const LIT_REAL: u8 = 34;
// frame
pub const FRM_LOAD: u8 = 40;
pub const FRM_STORE: u8 = 41;
pub const FRM_SWAP: u8 = 42;
pub const FRM_COPY: u8 = 43;
pub const FRM_POP: u8 = 44;
// list
pub const LIST_CREATE: u8 = 50;
pub const LIST_PUSH: u8 = 51;
pub const LIST_POP: u8 = 52;
pub const LIST_DOWNGRADE: u8 = 53;
pub const LIST_UPGRADE: u8 = 54;
// bytes
pub const BYTES_CREATE: u8 = 55;
// string
pub const STR_CREATE: u8 = 60;
pub const STR_CHAR_AT: u8 = 61;
pub const STR_CHARS: u8 = 62;
// seq
pub const SEQ_GET: u8 = 70;
pub const SEQ_SET: u8 = 71;
pub const SEQ_GET_SLICE: u8 = 72;
pub const SEQ_SET_SLICE: u8 = 73;
pub const SEQ_APPEND: u8 = 74;
pub const SEQ_LEN: u8 = 75;
pub const SEQ_RESIZE: u8 = 76;

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
    RealToInt,
    Cmp,
    // call and jump
    Call(u8),
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
