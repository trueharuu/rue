//! Global registry struct for runtime command lookup.

use std::{collections::HashMap};

use crate::command::core::traits::Command;

/// A registry that maps command names and aliases to their handlers.
pub struct Registry {
    /// The list of registered commands.
    commands: Vec<Box<dyn Command>>,
    /// A mapping from command names and aliases to their index in `commands`.
    index: HashMap<String, usize>,
}


impl Registry {
    /// Create an empty registry.
    #[must_use]
    pub fn new() -> Self {
        Self {
            commands: Vec::new(),
            index: HashMap::new(),
        }
    }

    /// Register a command. Its primary name and all aliases are indexed.
    pub fn register(&mut self, cmd: Box<dyn Command>) {
        let idx = self.commands.len();
        let meta = cmd.metadata();
        self.commands.push(cmd);
        self.index.insert(meta.name.to_lowercase(), idx);
        for alias in meta.aliases {
            self.index.insert(alias.to_lowercase(), idx);
        }
    }

    /// Look up a command by name or alias (case-insensitive).
    #[must_use]
    pub fn find(&self, name: &str) -> Option<&dyn Command> {
        self.index
            .get(&name.to_lowercase())
            .map(|&idx| &*self.commands[idx])
    }

    /// Return an iterator over all registered commands (deduplicated).
    pub fn iter(&self) -> impl Iterator<Item = &dyn Command> {
        self.commands.iter().map(|c| &**c)
    }
}

impl Default for Registry {
    fn default() -> Self {
        Self::new()
    }
}
