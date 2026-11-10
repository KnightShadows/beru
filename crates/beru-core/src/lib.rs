#![warn(missing_docs)]

//! Beru core engine — ABI profiles, compiler detection, and global cache.
//!
//! This crate provides the foundational services that `beru-build`,
//! `beru-recipe`, and `beru-cli` all depend on:
//!
//! - **ABI Profile**: detects the local C++ toolchain and computes a hash
//!   for cache keying
//! - **Cache**: content-addressed global cache at `~/.beru/`
//! - **Toolchain probing**: detects compilers, standard libraries, and
//!   architecture

/// ABI hashing and profiling logic.
pub mod abi;
/// Global cache and directory management.
pub mod cache;
/// Filesystem and byte-level utility helpers.
pub mod fs;
/// Compiler toolchain detection.
pub mod toolchain;
