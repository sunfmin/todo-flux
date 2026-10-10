//! The todo list and its on-disk text format.
//!
//! Flux tracks the list's length in its type, so every index handed to
//! `get`, `set_done` or `remove` is proved in bounds at compile time. The
//! only way to obtain such an index from untrusted input is `locate`.

use flux_rs::attrs::*;

const DONE_MARK: &str = "[x] ";
const OPEN_MARK: &str = "[ ] ";
pub struct Todo {
    pub title: String,
    pub done: bool,
}

impl Todo {
    /// The todo as one line of the text format: `[x] title` or `[ ] title`.
    pub fn line(&self) -> String {
        let mark = if self.done { DONE_MARK } else { OPEN_MARK };
        format!("{mark}{}", self.title)
    }

    /// Inverse of `line`. A line without a mark is an open todo, so any
    /// hand-edited file still loads.
    fn from_line(line: &str) -> Todo {
        let (done, title) = match line.strip_prefix(DONE_MARK) {
            Some(title) => (true, title),
            None => (false, line.strip_prefix(OPEN_MARK).unwrap_or(line)),
        };
        Todo {
            title: title.to_string(),
            done,
        }
    }
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
        self.push(Todo { title, done: false });
    }

    /// Turns the 1-based position a user types into a proved-valid index,
    /// or `None` when no todo sits at that position.
    #[spec(fn(&TodoList[@n], pos: usize) -> Option<usize{i: i < n}>)]
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

    /// Moves a todo to another position. The list keeps its length: nothing
    /// is lost or duplicated on the way.
    #[spec(fn(self: &mut TodoList[@n], from: usize{from < n}, to: usize{to < n}) ensures self: TodoList[n])]
    pub fn move_to(&mut self, from: usize, to: usize) {
        let todo = self.items.remove(from);
        self.items.insert(to, todo);
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
