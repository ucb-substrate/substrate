use super::{Error, TypeValidatorOutput};
use crate::digital::types::{HardwareType, TypeError};
use crate::digital::wire::{Op, WireDbInner, WireInner, WireValue};

impl WireInner {
    pub(crate) fn validate(&self, db: &WireDbInner, output: &mut TypeValidatorOutput) {
        match &self.value {
            WireValue::BinOp(_op, a, b) => {
                let a = &db[*a];
                let b = &db[*b];
                a.validate(db, output);
                b.validate(db, output);
                if a.hw_type != b.hw_type {
                    output.errors.push(Error::new(TypeError::InvalidOperandType(
                        Op::Add,
                        a.hw_type.clone(),
                        b.hw_type.clone(),
                    )));
                }
                match a.hw_type.get_uint() {
                    Some(uint_a) => match self.hw_type.get_uint() {
                        None => {
                            output.errors.push(Error::new(TypeError::InvalidType(
                                self.hw_type.clone(),
                                a.hw_type.clone(),
                            )));
                        }
                        Some(uint) => {
                            if uint_a.width() != uint.width() {
                                output
                                    .errors
                                    .push(Error::new(TypeError::InvalidOperandWidth(
                                        uint.width(),
                                        uint.width(),
                                        uint_a.width(),
                                    )));
                            }
                        }
                    },
                    None => {
                        output.errors.push(Error::new(TypeError::InvalidOperandType(
                            Op::Add,
                            a.hw_type.clone(),
                            b.hw_type.clone(),
                        )));
                    }
                }
            }
            WireValue::Literal(val) => match self.hw_type {
                HardwareType::UInt(uint) => {
                    if val.len() > uint.width() {
                        output.errors.push(Error::new(TypeError::InvalidLiteral(
                            val.clone(),
                            self.hw_type.clone(),
                        )));
                    }
                }
                _ => {
                    todo!()
                }
            },
            WireValue::Cast(_, val) => {
                db[*val].validate(db, output);
            }
            WireValue::Concat(a, b) => {
                db[*a].validate(db, output);
                db[*b].validate(db, output);
            }
            WireValue::Slice(_, val) => {
                db[*val].validate(db, output);
            }
            _ => {}
        }
    }
}
