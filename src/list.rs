//! The todo list and its on-disk text format.
//!
//! Flux tracks the list's length in its type, so every index handed to
//! `get`, `set_done` or `remove` is proved in bounds at compile time. The
//! only way to obtain such an index from untrusted input is `locate`.

use flux_rs::{attrs::*, defs, macros::invariant};

const DONE_MARK: &str = "[x] ";
const OPEN_MARK: &str = "[ ] ";
const PRIORITY_TAG: char = '!';

pub const HIGHEST_PRIORITY: u8 = 1;
pub const LOWEST_PRIORITY: u8 = 3;
const DEFAULT_PRIORITY: Priority = 2;

// The rules of this module, named once and used by the specs below.
defs! {
    fn is_priority(p: int) -> bool { HIGHEST_PRIORITY <= p && p <= LOWEST_PRIORITY }
    fn is_position(pos: int, len: int) -> bool { 1 <= pos && pos <= len }
}

/// How urgent a todo is. Flux only lets a value in the valid range have this
/// type, so the only way to make one from an arbitrary number is `priority`.
#[alias(type Priority = u8{p: is_priority(p)})]
pub type Priority = u8;

/// Checks a number typed by a user (or found in a file) against the range:
/// `Some` exactly when it is a priority, and then the same number.
// Flux reasons about the comparisons; it has no spec for `RangeInclusive::contains`.
#[allow(clippy::manual_range_contains)]
#[spec(fn(n: u8) -> Option<u8[n]>[is_priority(n)])]
pub fn priority(n: u8) -> Option<Priority> {
    if HIGHEST_PRIORITY <= n && n <= LOWEST_PRIORITY {
        Some(n)
    } else {
        None
    }
}

#[refined_by()]
pub struct Todo {
    pub title: String,
    pub done: bool,
    #[field(Priority)]
    pub priority: Priority,
}

impl Todo {
    /// The todo as one line of the text format, e.g. `[x] !2 title`.
    pub fn line(&self) -> String {
        let mark = if self.done { DONE_MARK } else { OPEN_MARK };
        format!("{mark}{PRIORITY_TAG}{} {}", self.priority, self.title)
    }

    /// Inverse of `line`. A missing mark means open and a missing or invalid
    /// priority means the default, so any hand-edited file still loads.
    fn from_line(line: &str) -> Todo {
        let (done, rest) = match line.strip_prefix(DONE_MARK) {
            Some(rest) => (true, rest),
            None => (false, line.strip_prefix(OPEN_MARK).unwrap_or(line)),
        };
        let (priority, title) = split_priority(rest);
        Todo {
            title: title.to_string(),
            done,
            priority,
        }
    }
}

/// Splits `!2 title` into its priority and title. Text without a valid
/// priority tag is all title.
#[spec(fn(&str) -> (Priority, &str))]
fn split_priority(rest: &str) -> (Priority, &str) {
    if let Some((tag, title)) = rest.split_once(' ')
        && let Some(digits) = tag.strip_prefix(PRIORITY_TAG)
        && let Ok(n) = digits.parse()
        && let Some(priority) = priority(n)
    {
        return (priority, title);
    }
    (DEFAULT_PRIORITY, rest)
}

/// `flux-alloc` has no specs for `Vec::remove` and `Vec::insert` yet. Only Flux reads this, so
/// the unstable `Allocator` it must name never reaches a normal build.
#[cfg(flux)]
mod vec_specs {
    use std::alloc::Allocator;

    use flux_rs::attrs::*;

    #[extern_spec]
    impl<T, A: Allocator> Vec<T, A> {
        #[spec(fn(self: &mut Vec<T, A>[@n], usize{i: i < n}) -> T ensures self: Vec<T, A>[n - 1])]
        fn remove(&mut self, index: usize) -> T;

        #[spec(fn(self: &mut Vec<T, A>[@n], usize{i: i <= n}, T) ensures self: Vec<T, A>[n + 1])]
        fn insert(&mut self, index: usize, element: T);
    }
}

/// How many of a number of todos are done. The invariant makes a count
/// with more done than there are impossible to build, so everything derived
/// from it is safe arithmetic.
#[refined_by(done: int, total: int)]
#[invariant(done <= total)]
pub struct Progress {
    #[field({usize[done] | done <= total})]
    pub done: usize,
    #[field(usize[total])]
    pub total: usize,
}

impl Progress {
    #[spec(fn(&Progress[@p]) -> usize[p.total - p.done])]
    pub fn left(&self) -> usize {
        self.total - self.done
    }

    /// The done share as a whole percentage. There is no share of nothing,
    /// so the caller must have at least one todo.
    #[spec(fn(&Progress[@p]) -> usize{v: v <= 100} requires p.total > 0)]
    pub fn percent(&self) -> usize {
        self.done * 100 / self.total
    }
}

#[refined_by(len: int)]
pub struct TodoList {
    #[field(Vec<Todo>[len])]
    items: Vec<Todo>,
}

impl TodoList {
    #[spec(fn() -> TodoList[0])]
    pub fn new() -> TodoList {
        TodoList { items: Vec::new() }
    }

    /// Reads the text format: one todo per line, blank lines ignored.
    pub fn parse(text: &str) -> TodoList {
        let mut list = TodoList::new();
        for line in text.lines() {
            if !line.trim().is_empty() {
                list.push(Todo::from_line(line));
            }
        }
        list
    }

    /// Writes the text format that `parse` reads.
    pub fn to_text(&self) -> String {
        let mut text = String::new();
        let mut i = 0;
        while i < self.len() {
            text.push_str(&self.get(i).line());
            text.push('\n');
            i += 1;
        }
        text
    }

    #[spec(fn(&TodoList[@n]) -> usize[n])]
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Appends an open todo. The title is kept on one line because a line is
    /// one todo in the text format.
    #[spec(fn(self: &mut TodoList[@n], title: &str) ensures self: TodoList[n + 1])]
    pub fn add(&mut self, title: &str) {
        let title = title.replace(['\n', '\r'], " ");
        self.push(Todo {
            title,
            done: false,
            priority: DEFAULT_PRIORITY,
        });
    }

    /// Turns the 1-based position a user types into its index: `Some` exactly
    /// when a todo sits at that position, and then the index just before it.
    #[spec(fn(&TodoList[@n], pos: usize) -> Option<usize[pos - 1]>[is_position(pos, n)])]
    pub fn locate(&self, pos: usize) -> Option<usize> {
        if 1 <= pos && pos <= self.len() {
            Some(pos - 1)
        } else {
            None
        }
    }

    #[spec(fn(&TodoList[@n], usize{i: i < n}) -> &Todo)]
    pub fn get(&self, index: usize) -> &Todo {
        &self.items[index]
    }

    #[spec(fn(&mut TodoList[@n], usize{i: i < n}, done: bool))]
    pub fn set_done(&mut self, index: usize, done: bool) {
        self.items[index].done = done;
    }

    #[spec(fn(&mut TodoList[@n], usize{i: i < n}, Priority))]
    pub fn set_priority(&mut self, index: usize, priority: Priority) {
        self.items[index].priority = priority;
    }

    /// Moves a todo to another position. The list keeps its length: nothing
    /// is lost or duplicated on the way.
    #[spec(fn(self: &mut TodoList[@n], from: usize{from < n}, to: usize{to < n}) ensures self: TodoList[n])]
    pub fn move_to(&mut self, from: usize, to: usize) {
        let todo = self.items.remove(from);
        self.items.insert(to, todo);
    }

    /// How much of the list is done.
    #[spec(fn(&TodoList[@n]) -> Progress{p: p.total == n})]
    pub fn progress(&self) -> Progress {
        let total = self.len();
        let mut done = 0;
        let mut i = 0;
        while i < total {
            invariant!(done: int; done <= i && i <= total);
            if self.get(i).done {
                done += 1;
            }
            i += 1;
        }
        Progress { done, total }
    }

    #[spec(fn(self: &mut TodoList[@n], usize{i: i < n}) -> Todo ensures self: TodoList[n - 1])]
    pub fn remove(&mut self, index: usize) -> Todo {
        self.items.remove(index)
    }

    #[spec(fn(self: &mut TodoList[@n], Todo) ensures self: TodoList[n + 1])]
    fn push(&mut self, todo: Todo) {
        self.items.push(todo);
    }
}
