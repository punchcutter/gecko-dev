/* Copyright 2018 Mozilla Foundation
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

//! This code bridges Spidermonkey to Cranelift.
//!
//! This documentation explains the role of each high-level function, each notable submodule, and
//! the Spidermonkey idiosyncrasies that are visible here and leak into Cranelift. This is not a
//! technical presentation of how Cranelift works or what it intends to achieve, a task much more
//! suited to the Wasmtime documentation itself:
//!
//! https://github.com/bytecodealliance/wasmtime/blob/master/cranelift/docs/index.md
//!
//! At the time of writing (April 14th, 2020), this code is only used for WebAssembly (wasm)
//! compilation, so this documentation focuses on the wasm integration. As a matter of fact, this
//! glue crate between Baldrmonkey and Cranelift is called Baldrdash, thanks to the usual punsters.
//!
//! ## Relationships to other files
//!
//! * WasmCraneliftCompile.cpp contains all the C++ code that calls into this crate.
//! * clifapi.h describes the C-style bindings to this crate's public functions, used by the C++
//! code to call into Rust. They're maintained by hand, and thus manual review must ensure the
//! signatures match those of the functions exposed in this lib.rs file.
//! * baldrapi.h describes the C-style functions exposed through `bindgen` so they can be called
//! from Rust. Bindings are automatically generated, such that they're safe to use in general.
//! WasmConstants.h is also exposed in through this file, which makes sharing some code easier.
//!
//! ## High-level functions
//!
//! * `cranelift_initialize` performs per-process initialization.
//! * `cranelift_compiler_create` will return a `BatchCompiler`, the high-level data structure
//! controlling the compilation of a group (batch) of wasm functions. The created compiler should
//! later be deallocated with `cranelift_compiler_destroy`, once it's not needed anymore.
//! * `cranelift_compile_function` takes care of translating a single wasm function into Cranelift
//! IR, and compiles it down to machine code. Input data is passed through a const pointer to a
//! `FuncCompilerInput` data structure (defined in bindings), and the return values are stored in
//! an in-out parameter named `CompiledFunc` (also defined in bindings).
//!
//! ## Submodules
//!
//! The list of submodules here is voluntarily put in a specific order, so as to make it easier to
//! discover and read.
//!
//! * The `isa` module configures Cranelift, applying some target-independent settings, as well as
//! target-specific settings. These settings are used both during translation of wasm to Cranelift
//! IR and compilation to machine code.
//! * The `wasm2clif` module contains the code doing the translation of the wasm code section to
//! Cranelift IR, implementing all the Spidermonkey specific behaviors.
//! * The `compile` module takes care of optimizing the Cranelift IR and compiles it down to
//! machine code, noting down relocations in the process.
//!
//! A few other helper modules are also defined:
//!
//! * The `bindings` module contains C++ bindings automatically generated by `bindgen` in the Cargo
//! build script (`build.rs`), as well as thin wrappers over these data structures to make these
//! more ergonomic to use in Rust.
//! * No code base would be feature complete without a bunch of random helpers and functions that
//! don't really belong anywhere else: the `utils` module contains error handling helpers, to unify
//! all the Cranelift Error types into one that can be passed around in Baldrdash.
//!
//! ## Spidermonkey idiosyncrasies
//!
//! Most of the Spidermonkey-specific behavior is reflected during conversion of the wasm code to
//! Cranelift IR (in the `wasm2clif` module), but there are some other aspects worth mentioning
//! here.
//!
//! ### Code generation, prologues/epilogues, ABI
//!
//! Cranelift may call into and be called from other functions using the Spidermonkey wasm ABI:
//! that is, code generated by the wasm baseline compiler during tiering, any other wasm stub, even
//! Ion (through the JIT entries and exits).
//!
//! As a matter of fact, it must push the same C++ `wasm::Frame` on the stack before a call, and
//! unwind it properly on exit. To keep this detail orthogonal to Cranelift, the function's
//! prologue and epilogue are **not** generated by Cranelift itself; the C++ code generates them
//! for us. Here, Cranelift only generates the code section and appropriate relocations.
//! The C++ code writes the prologue, copies the machine code section, writes the epilogue, and
//! translates the Cranelift relocations into Spidermonkey relocations.
//!
//! * To not generate the prologue and epilogue, Cranelift uses a special calling convention called
//! Baldrdash in its code. This is set upon creation of the `TargetISA`.
//! * Cranelift must know the offset to the stack argument's base, that is, the size of the
//! wasm::Frame. The `baldrdash_prologue_words` setting is used to propagate this information to
//! Cranelift.
//! * Since Cranelift generated functions interact with Ion-ABI functions (Ionmonkey, other wasm
//! functions), and native (host) functions, it has to respect both calling conventions. Especially
//! when it comes to function calls it must preserve callee-saved and caller-saved registers in a
//! way compatible with both ABIs. In practice, it means Cranelift must consider Ion's callee-saved
//! as its callee-saved, and native's caller-saved as its caller-saved (since it deals with both
//! ABIs, it has to union the sets).
//!
//! ### Maintaining HeapReg
//!
//! On some targets, Spidermonkey pins one register to keep the heap-base accessible at all-times,
//! making memory accesses cheaper. This register is excluded from Ion's register allocation, and
//! is manually maintained by Spidermonkey before and after calls.
//!
//! Cranelift has two settings to mimic the same behavior:
//! - `enable_pinned_reg` makes it possible to pin a register and gives access to two Cranelift
//! instructions for reading it and writing to it.
//! - `use_pinned_reg_as_heap_base` makes the code generator use the pinned register as the heap
//! base for all Cranelift IR memory accesses.
//!
//! Using both settings allows to reproduce Spidermonkey's behavior. One caveat is that the pinned
//! register used in Cranelift must match the HeapReg register in Spidermonkey, for this to work
//! properly.
//!
//! Not using the pinned register as the heap base, when there's a heap register on the platform,
//! means that we have to explicitly maintain it in the prologue and epilogue (because of tiering),
//! which would be another source of slowness.
//!
//! ### Non-streaming validation
//!
//! Ionmonkey is able to iterate over the wasm code section's body, validating and emitting the
//! internal Ionmonkey's IR at the same time.
//!
//! Cranelift uses `wasmparser` to parse the wasm binary section, which isn't able to add
//! per-opcode hooks. Instead, Cranelift validates (off the main thread) the function's body before
//! compiling it, function per function.

mod bindings;
mod compile;
mod isa;
mod utils;
mod wasm2clif;

use log::{self, error, info};
use std::ptr;

use crate::bindings::{CompiledFunc, FuncCompileInput, ModuleEnvironment, StaticEnvironment};
use crate::compile::BatchCompiler;
use cranelift_codegen::CodegenError;

/// Initializes all the process-wide Cranelift state. It must be called at least once, before any
/// other use of this crate. It is not an issue if it is called more than once; subsequent calls
/// are useless though.
#[no_mangle]
pub extern "C" fn cranelift_initialize() {
    // Gecko might set a logger before we do, which is all fine; try to initialize ours, and reset
    // the FilterLevel env_logger::try_init might have set to what it was in case of initialization
    // failure
    let filter = log::max_level();
    match env_logger::try_init() {
        Ok(_) => {}
        Err(_) => {
            log::set_max_level(filter);
        }
    }
}

/// Allocate a compiler for a module environment and return an opaque handle.
///
/// It is the caller's responsability to deallocate the returned BatchCompiler later, passing back
/// the opaque handle to a call to `cranelift_compiler_destroy`.
///
/// This is declared in `clifapi.h`.
#[no_mangle]
pub unsafe extern "C" fn cranelift_compiler_create<'a, 'b>(
    static_env: *const StaticEnvironment,
    env: *const bindings::LowLevelModuleEnvironment,
) -> *mut BatchCompiler<'a, 'b> {
    let env = env.as_ref().unwrap();
    let static_env = static_env.as_ref().unwrap();
    match BatchCompiler::new(static_env, ModuleEnvironment::new(env)) {
        Ok(compiler) => Box::into_raw(Box::new(compiler)),
        Err(err) => {
            error!("When constructing the batch compiler: {}", err);
            ptr::null_mut()
        }
    }
}

/// Deallocate a BatchCompiler created by `cranelift_compiler_create`.
///
/// Passing any other kind of pointer to this function is technically undefined behavior, thus
/// making the function unsafe to use.
///
/// This is declared in `clifapi.h`.
#[no_mangle]
pub unsafe extern "C" fn cranelift_compiler_destroy(compiler: *mut BatchCompiler) {
    assert!(
        !compiler.is_null(),
        "NULL pointer passed to cranelift_compiler_destroy"
    );
    // Convert the pointer back into the box it came from. Then drop it.
    let _box = Box::from_raw(compiler);
}

/// Compile a single function.
///
/// This is declared in `clifapi.h`.
#[no_mangle]
pub unsafe extern "C" fn cranelift_compile_function(
    compiler: *mut BatchCompiler,
    data: *const FuncCompileInput,
    result: *mut CompiledFunc,
) -> bool {
    let compiler = compiler.as_mut().unwrap();
    let data = data.as_ref().unwrap();

    // Reset the compiler to a clean state.
    compiler.clear();

    if let Err(e) = compiler.translate_wasm(data) {
        error!("Wasm translation error: {}\n", e);
        info!("Translated function: {}", compiler);
        return false;
    };

    if let Err(e) = compiler.compile(data.stackmaps()) {
        // Make sure to panic on verifier errors, so that fuzzers see those. Other errors are about
        // unsupported features or implementation limits, so just report them as a user-facing
        // error.
        match e {
            CodegenError::Verifier(verifier_error) => {
                panic!("Cranelift verifier error: {}", verifier_error);
            }
            CodegenError::ImplLimitExceeded
            | CodegenError::CodeTooLarge
            | CodegenError::Unsupported(_) => {
                error!("Cranelift compilation error: {}\n", e);
                info!("Compiled function: {}", compiler);
                return false;
            }
        }
    };

    // TODO(bbouvier) if destroy is called while one of these objects is alive, you're going to
    // have a bad time. Would be nice to be able to enforce lifetimes accross languages, somehow.
    let result = result.as_mut().unwrap();
    result.reset(&compiler.current_func);

    true
}

/// Returns true whether a platform (target ISA) is supported or not.
#[no_mangle]
pub unsafe extern "C" fn cranelift_supports_platform() -> bool {
    isa::platform::IS_SUPPORTED
}
