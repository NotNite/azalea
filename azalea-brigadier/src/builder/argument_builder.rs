use parking_lot::RwLock;

use crate::{
    context::CommandContext,
    modifier::RedirectModifier,
    tree::{Command, CommandNode},
};

use super::{literal_argument_builder::Literal, required_argument_builder::Argument};
use std::{fmt::Debug, sync::Arc};

#[derive(Debug, Clone)]
pub enum ArgumentBuilderType {
    Literal(Literal),
    Argument(Argument),
}

/// A node that hasn't yet been built.
pub struct ArgumentBuilder<S> {
    arguments: CommandNode<S>,

    command: Command<S>,
    requirement: Arc<dyn Fn(Arc<S>) -> bool + Send + Sync>,
    target: Option<Arc<RwLock<CommandNode<S>>>>,

    forks: bool,
    modifier: Option<Arc<RedirectModifier<S>>>,
}

/// A node that isn't yet built.
impl<S> ArgumentBuilder<S> {
    pub fn new(value: ArgumentBuilderType) -> Self {
        Self {
            arguments: CommandNode {
                value,
                ..Default::default()
            },
            command: None,
            requirement: Arc::new(|_| true),
            forks: false,
            modifier: None,
            target: None,
        }
    }

    pub fn then(self, argument: ArgumentBuilder<S>) -> Self {
        self.then_built(argument.build())
    }

    pub fn then_built(mut self, argument: CommandNode<S>) -> Self {
        self.arguments.add_child(&Arc::new(RwLock::new(argument)));
        self
    }

    pub fn executes<F>(mut self, f: F) -> Self
    where
        F: Fn(&CommandContext<S>) -> i32 + Send + Sync + 'static,
    {
        self.command = Some(Arc::new(f));
        self
    }

    pub fn requires<F>(mut self, requirement: F) -> Self
    where
        F: Fn(Arc<S>) -> bool + Send + Sync + 'static,
    {
        self.requirement = Arc::new(requirement);
        self
    }

    pub fn redirect(self, target: Arc<RwLock<CommandNode<S>>>) -> Self {
        self.forward(target, None, false)
    }

    pub fn fork(
        self,
        target: Arc<RwLock<CommandNode<S>>>,
        modifier: Arc<RedirectModifier<S>>,
    ) -> Self {
        self.forward(target, Some(modifier), true)
    }

    pub fn forward(
        mut self,
        target: Arc<RwLock<CommandNode<S>>>,
        modifier: Option<Arc<RedirectModifier<S>>>,
        fork: bool,
    ) -> Self {
        if !self.arguments.children.is_empty() {
            panic!("Cannot forward a node with children");
        }
        self.target = Some(target);
        self.modifier = modifier;
        self.forks = fork;
        self
    }

    pub fn build(self) -> CommandNode<S> {
        let mut result = CommandNode {
            value: self.arguments.value,
            command: self.command,
            requirement: self.requirement,
            redirect: self.target,
            modifier: self.modifier,
            forks: self.forks,
            arguments: Default::default(),
            children: Default::default(),
            literals: Default::default(),
        };

        for argument in self.arguments.children.values() {
            result.add_child(argument);
        }

        result
    }
}

impl<S> Debug for ArgumentBuilder<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ArgumentBuilder")
            .field("arguments", &self.arguments)
            // .field("command", &self.command)
            // .field("requirement", &self.requirement)
            .field("target", &self.target)
            .field("forks", &self.forks)
            // .field("modifier", &self.modifier)
            .finish()
    }
}
