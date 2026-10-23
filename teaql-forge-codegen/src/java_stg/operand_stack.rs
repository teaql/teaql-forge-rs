//! ST4 4.3.3 keeps 100 physical slots, including values above its signed sp.
use super::operands::{Operand, fault};
use serde_json::Value;

pub(super) struct OperandStack {
    slots: Vec<Operand>,
    pub pointer: isize,
}

impl Default for OperandStack {
    fn default() -> Self {
        Self {
            slots: vec![Operand::Value(Value::Null); 100],
            pointer: -1,
        }
    }
}

impl OperandStack {
    pub fn at(&self, position: isize) -> Result<&Operand, String> {
        usize::try_from(position)
            .ok()
            .and_then(|p| self.slots.get(p))
            .ok_or_else(fault)
    }
    pub fn at_mut(&mut self, position: isize) -> Result<&mut Operand, String> {
        usize::try_from(position)
            .ok()
            .and_then(|p| self.slots.get_mut(p))
            .ok_or_else(fault)
    }
    pub fn set(&mut self, position: isize, value: Operand) -> Result<(), String> {
        *self.at_mut(position)? = value;
        Ok(())
    }
    pub fn push(&mut self, value: Operand) -> Result<(), String> {
        let position = self.reserve();
        self.set(position, value)
    }
    /// Evaluate ++sp before an assignment's fallible right-hand side.
    pub fn reserve(&mut self) -> isize {
        self.pointer += 1;
        self.pointer
    }
    pub fn pop(&mut self) -> Result<Operand, String> {
        let position = self.pointer;
        self.pointer -= 1;
        self.at(position).cloned()
    }
    pub fn top_mut(&mut self) -> Result<&mut Operand, String> {
        self.at_mut(self.pointer)
    }
    pub fn drop(&mut self, count: usize) {
        self.pointer -= count as isize;
    }
}
