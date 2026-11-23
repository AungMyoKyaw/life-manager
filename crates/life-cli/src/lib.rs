//! Shared library surface of the Life Manager CLI.
//!
//! The MCP tool catalog lives here so the binary, the contract tests and any
//! future embedder all read exactly the same specification.
pub mod action_spec;
pub mod remote_mcp;
