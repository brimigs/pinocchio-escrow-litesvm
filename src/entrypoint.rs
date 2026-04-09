#[cfg(feature = "bpf-entrypoint")]
use pinocchio::{default_allocator, nostd_panic_handler, program_entrypoint};

#[cfg(feature = "bpf-entrypoint")]
program_entrypoint!(crate::processor::process_instruction);

#[cfg(feature = "bpf-entrypoint")]
default_allocator!();

#[cfg(feature = "bpf-entrypoint")]
nostd_panic_handler!();
