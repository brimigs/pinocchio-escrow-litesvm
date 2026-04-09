#![no_std]

extern crate alloc;

use pinocchio::address::declare_id;

pub mod entrypoint;
pub mod error;
pub mod instruction;
pub mod processor;
pub mod state;

declare_id!("7h1hJkP8i1H2Q7xPkD8STGvN6dEyxwCxPj3YfJ8p6r7T");
