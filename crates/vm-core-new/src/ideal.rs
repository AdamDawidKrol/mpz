//! Ideal backend for symbolic execution.

use std::collections::BTreeMap;

use ir::{BlockId, Function};
use mpz_common::Context as IoContext;
use serio::{SinkExt, stream::IoStreamExt};

use crate::{
    Directive, Global, Module, Operand, Param, VmError,
    bitset::BitSet,
    call::Call,
    host::{HostState, MemoryInput, MemoryInputKind},
    thread::{Context, StepResult, Thread},
    value::Value,
};

#[derive(Debug, Clone)]
pub enum TraceEvent {
    Directive(Directive),
    PrivateControlFlowStart { func_idx: u32, block: BlockId },
    PrivateControlFlowEnd,
}

pub struct Instance {
    module: Module,
    global: Global,
    host_state: HostState,
    /// Private memory to send to peer: ptr -> bytes.
    private_memory: BTreeMap<u32, Vec<u8>>,
    /// Blind memory to receive from peer: ptr -> len.
    blind_memory: BTreeMap<u32, usize>,
    /// Optional trace log for profiling.
    trace_log: Option<Vec<TraceEvent>>,
    /// Local mode: all values available locally, no I/O needed.
    local: bool,
}

impl Instance {
    pub fn new(module: Module) -> Result<Self, VmError> {
        crate::imports::validate_imports(&module)?;
        let global = Global::new(&module)?;
        Ok(Self {
            module,
            global,
            host_state: HostState::default(),
            private_memory: BTreeMap::new(),
            blind_memory: BTreeMap::new(),
            trace_log: None,
            local: false,
        })
    }

    pub fn new_local(module: Module) -> Result<Self, VmError> {
        let mut inst = Self::new(module)?;
        inst.local = true;
        Ok(inst)
    }

    pub fn with_tracing(module: Module) -> Result<Self, VmError> {
        let mut inst = Self::new(module)?;
        inst.trace_log = Some(Vec::new());
        Ok(inst)
    }

    pub fn module(&self) -> &Module {
        &self.module
    }
    pub fn global(&self) -> &Global {
        &self.global
    }
    pub fn global_mut(&mut self) -> &mut Global {
        &mut self.global
    }
    pub fn host_state(&self) -> &HostState {
        &self.host_state
    }
    pub fn host_state_mut(&mut self) -> &mut HostState {
        &mut self.host_state
    }
    pub fn trace_log(&self) -> Option<&[TraceEvent]> {
        self.trace_log.as_deref()
    }

    pub fn mark_private(&mut self, ptr: u32, len: usize) -> Result<(), VmError> {
        if self.host_state.memory_inputs.contains_key(&ptr) {
            return Err(VmError::DuplicateMemoryInput { ptr });
        }
        self.global.memory_taint_mut().insert_range(ptr, len);
        self.host_state.memory_inputs.insert(
            ptr,
            MemoryInput {
                ptr,
                len,
                kind: MemoryInputKind::Private,
                initialized: false,
            },
        );
        Ok(())
    }

    pub fn mark_blind(&mut self, ptr: u32, len: usize) -> Result<(), VmError> {
        if self.host_state.memory_inputs.contains_key(&ptr) {
            return Err(VmError::DuplicateMemoryInput { ptr });
        }
        self.global.memory_taint_mut().insert_range(ptr, len);
        self.host_state.memory_inputs.insert(
            ptr,
            MemoryInput {
                ptr,
                len,
                kind: MemoryInputKind::Blind,
                initialized: false,
            },
        );
        self.blind_memory.insert(ptr, len);
        Ok(())
    }

    pub fn write_private(&mut self, ptr: u32, data: &[u8]) -> Result<(), VmError> {
        let input = self
            .host_state
            .memory_inputs
            .get(&ptr)
            .ok_or(VmError::UnknownMemoryInput { ptr })?;
        if input.len != data.len() {
            return Err(VmError::MemoryInputLengthMismatch {
                ptr,
                expected: input.len,
                got: data.len(),
            });
        }
        if !matches!(input.kind, MemoryInputKind::Private) {
            return Err(VmError::Internal(format!(
                "cannot write_private to blind region at {:#x}",
                ptr
            )));
        }
        // Write directly to Global memory so Thread can read it.
        let memory = self.global.memory_mut().ok_or(VmError::MemoryNotDefined)?;
        memory.write_bytes(ptr, data)?;
        self.private_memory.insert(ptr, data.to_vec());
        Ok(())
    }

    fn validate_call(
        &self,
        func_idx: u32,
        params: &[Param],
    ) -> Result<&ir::LocalFunction, VmError> {
        let func = self
            .module
            .function(func_idx)
            .ok_or(VmError::UndefinedFunction(func_idx))?;
        let func = match func {
            Function::Import(_) => return Err(VmError::InvalidFunction(func_idx)),
            Function::Local(func) => func,
        };
        if func.func_type().results.len() > 1 {
            return Err(VmError::Unsupported("multi-value return".into()));
        }
        if params.len() != func.func_type().params.len() {
            todo!()
        }
        for (arg, expected_ty) in params.iter().zip(&func.func_type().params) {
            if &arg.ty() != expected_ty {
                return Err(VmError::TypeMismatch {
                    expected: *expected_ty,
                    got: arg.ty(),
                });
            }
        }
        Ok(func)
    }

    pub fn call_sync(
        &mut self,
        func_idx: u32,
        params: Vec<Param>,
    ) -> Result<Option<Value>, VmError> {
        self.validate_call(func_idx, &params)?;
        if self.local {
            // Reject Blind params in local mode.
            if params.iter().any(|p| matches!(p, Param::Blind(_))) {
                return Err(VmError::Internal(
                    "Blind params not supported in local mode".into(),
                ));
            }
        }
        let call = Call { func_idx, params };
        let mut thread = Thread::new();
        {
            let mut cx = Context {
                module: &self.module,
                global: &mut self.global,
            };
            thread.call(&mut cx, call)?;
        }
        let private_eval = true;
        loop {
            let result = {
                let mut cx = Context {
                    module: &self.module,
                    global: &mut self.global,
                };
                match thread.step(&mut cx, private_eval) {
                    Ok(r) => r,
                    Err(e) => {
                        self.dump_error(&thread, &e);
                        return Err(e);
                    }
                }
            };
            match result {
                StepResult::Continue => {}
                StepResult::Directive(directive) => {
                    if !self.local {
                        match &directive {
                            Directive::Branch {
                                cond: Some(Operand::Symbol(_)),
                                ..
                            } => {
                                return Err(VmError::Blocked);
                            }
                            Directive::Call { func_idx, .. }
                                if matches!(
                                    self.module.function(*func_idx),
                                    Some(Function::Import(_))
                                ) =>
                            {
                                return Err(VmError::Blocked);
                            }
                            _ => {}
                        }
                    }
                    // In local mode, resolve host calls immediately.
                    if let Directive::Call { dst, func_idx, .. } = &directive {
                        if matches!(self.module.function(*func_idx), Some(Function::Import(_))) {
                            let value = dst.map(|_| Value::from(0i32));
                            thread.resolve_call(value)?;
                        }
                    }
                    if let Some(log) = &mut self.trace_log {
                        log.push(TraceEvent::Directive(directive));
                    }
                }
                StepResult::Done { result } => return Ok(result),
            }
        }
    }

    pub async fn call(
        &mut self,
        io: &mut IoContext,
        func_idx: u32,
        params: Vec<Param>,
    ) -> Result<Option<Value>, VmError> {
        self.validate_call(func_idx, &params)?;
        let call = Call {
            func_idx,
            params: params.clone(),
        };
        let mut thread = Thread::new();
        {
            let mut cx = Context {
                module: &self.module,
                global: &mut self.global,
            };
            thread.call(&mut cx, call)?;
        }
        self.flush(&mut thread, io, &params).await?;
        self.run_loop(&mut thread).await
    }

    pub async fn call_with_decode(
        &mut self,
        io: &mut IoContext,
        func_idx: u32,
        params: Vec<Param>,
    ) -> Result<Option<Value>, VmError> {
        self.validate_call(func_idx, &params)?;
        let call = Call {
            func_idx,
            params: params.clone(),
        };
        let mut thread = Thread::new();
        {
            let mut cx = Context {
                module: &self.module,
                global: &mut self.global,
            };
            thread.call(&mut cx, call)?;
        }
        self.flush(&mut thread, io, &params).await?;
        self.run_loop(&mut thread).await
    }

    /// Exchange private/blind values with peer and write resolved values
    /// into Thread's registers.
    async fn flush(
        &mut self,
        thread: &mut Thread,
        io: &mut IoContext,
        params: &[Param],
    ) -> Result<(), VmError> {
        let has_pending_params = params
            .iter()
            .any(|p| matches!(p, Param::Private(_) | Param::Blind(_)));
        let has_pending_memory =
            !self.private_memory.is_empty() || !self.blind_memory.is_empty();

        if !has_pending_params && !has_pending_memory {
            return Ok(());
        }

        // Exchange param register values.
        let to_send: BTreeMap<(usize, u32), Value> = params
            .iter()
            .enumerate()
            .filter_map(|(i, p)| {
                if let Param::Private(v) = p {
                    Some(((0, i as u32), *v))
                } else {
                    None
                }
            })
            .collect();

        io.io_mut().send(to_send).await?;
        let received: BTreeMap<(usize, u32), Value> = io.io_mut().expect_next().await?;

        let thread_base = thread
            .call_stack()
            .last()
            .map(|f| f.reg_base())
            .unwrap_or(0) as usize;
        let thread_regs = thread.registers_mut();
        for (i, p) in params.iter().enumerate() {
            if let Param::Blind(_) = p {
                let value = received[&(0, i as u32)];
                if thread_base + i < thread_regs.len() {
                    thread_regs[thread_base + i] = value;
                }
            }
        }

        // Exchange private memory regions.
        let mem_to_send: BTreeMap<u32, Vec<u8>> = self.private_memory.clone();
        io.io_mut().send(mem_to_send).await?;
        let received_mem: BTreeMap<u32, Vec<u8>> = io.io_mut().expect_next().await?;

        let memory = self.global.memory_mut().ok_or(VmError::MemoryNotDefined)?;
        for (ptr, data) in &received_mem {
            memory.write_bytes(*ptr, data)?;
        }

        Ok(())
    }

    fn dump_error(&self, thread: &Thread, err: &VmError) {
        eprintln!("VM error: {err:?}");
        eprintln!("Call stack:");
        for (i, frame) in thread.call_stack().iter().rev().enumerate() {
            let name = self
                .module
                .function_names()
                .get(&frame.func_idx())
                .map(|s| s.as_str())
                .unwrap_or("<unknown>");
            eprintln!(
                "  {i}: {name} (func_idx={}, block={:?}, ip={})",
                frame.func_idx(),
                frame.current_block(),
                frame.ip()
            );
        }
    }

    async fn run_loop(&mut self, thread: &mut Thread) -> Result<Option<Value>, VmError> {
        let mut was_in_private_cf = thread.in_private_cf();

        loop {
            let result = {
                let mut cx = Context {
                    module: &self.module,
                    global: &mut self.global,
                };
                match thread.step(&mut cx, true) {
                    Ok(r) => r,
                    Err(e) => {
                        self.dump_error(&thread, &e);
                        return Err(e);
                    }
                }
            };

            match result {
                StepResult::Continue => {}
                StepResult::Directive(directive) => {
                    if let Some(log) = &mut self.trace_log {
                        log.push(TraceEvent::Directive(directive.clone()));
                    }
                    match &directive {
                        Directive::Call { func_idx, .. }
                            if matches!(
                                self.module.function(*func_idx),
                                Some(Function::Import(_))
                            ) =>
                        {
                            todo!("handle host call in run_loop")
                        }
                        _ => {}
                    }
                }
                StepResult::Done { result } => return Ok(result),
            }

            // Track private CF transitions for trace events.
            let in_private_cf = thread.in_private_cf();
            if in_private_cf && !was_in_private_cf {
                if let Some(log) = &mut self.trace_log {
                    if let Some(f) = thread.call_stack().last() {
                        log.push(TraceEvent::PrivateControlFlowStart {
                            func_idx: f.func_idx(),
                            block: f.current_block(),
                        });
                    }
                }
            }
            if !in_private_cf && was_in_private_cf {
                if let Some(log) = &mut self.trace_log {
                    log.push(TraceEvent::PrivateControlFlowEnd);
                }
            }
            was_in_private_cf = in_private_cf;
        }
    }
}
