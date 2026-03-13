//! Rune File I/O Module
//!
//! This module provides a Rune native module that exposes file system operations
//! to rune blocks. It is installed in both the executor (runtime) and the type
//! checker (compilation) to allow rune blocks to read and write files.
//!
//! The module is registered under the `fs` crate name to avoid conflicts with
//! Rune's built-in `file!()` macro namespace.
//!
//! # Available Functions
//!
//! - `fs::write(path, content) -> bool` — Write content to a file; returns true on success
//! - `fs::read(path) -> String` — Read file contents; returns empty string on failure
//! - `fs::append(path, content) -> bool` — Append content to a file; returns true on success
//! - `env::var(name) -> String` — Read an environment variable; returns empty string if not set

/// Build and return the file I/O Rune module.
///
/// The module is registered under the `fs` crate name so that rune block
/// code can call `fs::write(...)`, `fs::read(...)`, and `fs::append(...)`.
pub fn file_io_module() -> Result<rune::Module, rune::ContextError> {
    let mut m = rune::Module::with_crate("fs")?;

    // fs::write(path, content) -> bool
    m.function("write", |path: String, content: String| -> bool {
        std::fs::write(&path, &content).is_ok()
    })
    .build()?;

    // fs::read(path) -> String
    m.function("read", |path: String| -> String {
        std::fs::read_to_string(&path).unwrap_or_default()
    })
    .build()?;

    // fs::append(path, content) -> bool
    m.function("append", |path: String, content: String| -> bool {
        use std::io::Write;
        std::fs::OpenOptions::new()
            .append(true)
            .create(true)
            .open(&path)
            .and_then(|mut f| f.write_all(content.as_bytes()))
            .is_ok()
    })
    .build()?;

    Ok(m)
}

/// Build and return the environment variable Rune module.
///
/// The module is registered under the `env` crate name so that rune block
/// code can call `env::var(...)` to read environment variables at runtime.
pub fn env_module() -> Result<rune::Module, rune::ContextError> {
    let mut m = rune::Module::with_crate("env")?;

    // env::var(name) -> String — returns empty string if not set
    m.function("var", |name: String| -> String {
        std::env::var(&name).unwrap_or_default()
    })
    .build()?;

    Ok(m)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_io_module_builds() {
        let module = file_io_module();
        assert!(module.is_ok(), "file_io_module should build without errors");
    }

    #[test]
    fn test_file_io_module_installs_in_context() {
        let mut context =
            rune::Context::with_default_modules().expect("failed to create default Rune context");
        let module = file_io_module().expect("file_io_module should build");
        let result = context.install(module);
        assert!(
            result.is_ok(),
            "file_io_module should install into Rune context without errors"
        );
    }

    #[test]
    fn test_env_module_builds() {
        let module = env_module();
        assert!(module.is_ok(), "env_module should build without errors");
    }

    #[test]
    fn test_env_module_installs_in_context() {
        let mut context =
            rune::Context::with_default_modules().expect("failed to create default Rune context");
        let module = env_module().expect("env_module should build");
        let result = context.install(module);
        assert!(
            result.is_ok(),
            "env_module should install into Rune context without errors"
        );
    }
}
