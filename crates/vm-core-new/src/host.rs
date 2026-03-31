use std::{collections::BTreeMap, task::Poll};

use ir::ValType;
use rangeset::set::RangeSet;

use crate::{VmError, value::Value};

#[derive(Debug, Default)]
pub(crate) struct HostState {
    decode_id: usize,
    decodes: BTreeMap<usize, DecodeInfo>,
    /// Tracked memory allocations (ptr -> size).
    allocations: BTreeMap<u32, u32>,
    /// Memory inputs (private/blind regions) - ptr -> input.
    pub(crate) memory_inputs: BTreeMap<u32, MemoryInput>,
}

impl HostState {
    pub(crate) fn alloc(&mut self, addr: u32, size: u32) {
        self.allocations.insert(addr, size);
    }

    pub(crate) fn free(&mut self, addr: u32) {
        self.allocations.remove(&addr);
    }

    /// Register a memory decode.
    pub(crate) fn register_mem_decode(&mut self, ptr: u32, len: usize) -> usize {
        let id = self.decode_id;
        self.decode_id += 1;
        self.decodes.insert(
            id,
            DecodeInfo {
                kind: DecodeKind::Memory { ptr, len },
                result: None,
            },
        );
        id
    }

    /// Register a value decode.
    pub(crate) fn register_value_decode(&mut self, ty: ValType) -> usize {
        let id = self.decode_id;
        self.decode_id += 1;
        self.decodes.insert(
            id,
            DecodeInfo {
                kind: DecodeKind::Value { ty },
                result: None,
            },
        );
        id
    }

    /// Resolve a decode operation.
    pub fn resolve_decode(&mut self, id: usize, value: Value) -> Result<(), VmError> {
        let info = self
            .decodes
            .get_mut(&id)
            .ok_or_else(|| VmError::Internal(format!("decode not registered ({id})")))?;
        info.result = Some(value);
        Ok(())
    }

    /// Get the result of a decode, if resolved.
    pub(crate) fn get_decode_result(&self, id: usize) -> Result<Option<Value>, VmError> {
        let info = self
            .decodes
            .get(&id)
            .ok_or_else(|| VmError::Internal(format!("decode not registered ({id})")))?;
        Ok(info.result)
    }

    /// Complete a memory decode - clears the symbol range if resolved.
    pub(crate) fn complete_mem_decode(&mut self, id: usize) -> Result<Poll<()>, VmError> {
        let info = self
            .decodes
            .get(&id)
            .ok_or_else(|| VmError::Internal(format!("decode not registered ({id})")))?;

        let DecodeKind::Memory { ptr, len } = info.kind else {
            return Err(VmError::Internal(format!(
                "decode {id} is not a memory decode"
            )));
        };

        if info.result.is_some() {
            self.decodes.remove(&id);
            Ok(Poll::Ready(()))
        } else {
            Ok(Poll::Pending)
        }
    }

    /// Check if reading from this address range would access an uninitialized
    /// memory input. Returns Some(ptr) if an uninitialized region is found,
    /// None otherwise.
    pub(crate) fn check_uninitialized_memory_input(&self, addr: u32, len: usize) -> Option<u32> {
        for (ptr, input) in &self.memory_inputs {
            if !input.initialized {
                // Check if [addr, addr+len) overlaps with [ptr, ptr+input.len)
                let input_end = ptr + input.len as u32;
                let read_end = addr + len as u32;
                if addr < input_end && read_end > *ptr {
                    return Some(*ptr);
                }
            }
        }
        None
    }
}

/// Kind of memory input for private/blind values through linear memory.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryInputKind {
    /// Private value - we have the bytes, peer needs them during flush.
    Private,
    /// Blind value - peer has the bytes, we need them during flush.
    Blind,
}

/// A memory region marked as private or blind input.
#[derive(Debug, Clone)]
pub struct MemoryInput {
    /// Starting address in linear memory.
    pub ptr: u32,
    /// Length of the region in bytes.
    pub len: usize,
    /// Whether this is private or blind.
    pub kind: MemoryInputKind,
    /// Whether the value has been written (initialized).
    pub initialized: bool,
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum DecodeKind {
    Memory {
        ptr: u32,
        len: usize,
    },
    Value {
        #[allow(dead_code)]
        ty: ValType,
    },
}

#[derive(Debug, Clone)]
pub(crate) struct DecodeInfo {
    kind: DecodeKind,
    result: Option<Value>,
}

/// A preprocessed function stored for later execution.
#[derive(Debug)]
pub struct PreprocessedFunction {
    /// Number of symbolic inputs (locals) expected.
    pub num_inputs: u32,
}
