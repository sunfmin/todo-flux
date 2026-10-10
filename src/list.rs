//! The todo list and its on-disk text format.
//!
//! Flux tracks the list's length in its type, so every index handed to
//! indexing, `set_status` or `remove` is proved in bounds at compile time. The
//! only way to obtain such an index from untrusted input is `locate`.

use std::ops::Index;

use flux_rs::{attrs::*, defs, macros::invariant};

const PRIORITY_TAG: char = '!';

pub const HIGHEST_PRIORITY: u8 = 1;
pub const LOWEST_PRIORITY: u8 = 3;
const DEFAULT_PRIORITY: Priority = 2;
pub const PRIORITY_LEVELS: usize = (LOWEST_PRIORITY - HIGHEST_PRIORITY + 1) as usize;

// The rules of this module, named once and used by the specs below.
defs! {
    fn is_priority(p: int) -> bool { HIGHEST_PRIORITY <= p && p <= LOWEST_PRIORITY }
    fn is_position(pos: int, len: int) -> bool { 1 <= pos && pos <= len }
    // A finished todo must be reopened before it can be worked on again.
    // Hidden: other code may rely on the rule holding, not on what it says,
    // so the rule can change without any other proof having to.
    #[hide]
    fn can_become(from: Status, to: Status) -> bool { !(from == Status::Done && to == Status::Doing) }
}

/// Where a todo stands. `reflect` lets the specs talk about the variants.
#[reflect]
#[derive(Clone, Copy, PartialEq)]
pub enum Status {
    Open,
    Doing,
    Done,
}

impl Status {
    const ALL: [Status; 3] = [Status::Open, Status::Doing, Status::Done];

    /// Whether a todo in this status may move to `to`. This is the one
    /// function that sees inside the rule, to be checked against it.
    #[reveal(can_become)]
    #[spec(fn(&Status[@from], to: Status) -> bool[can_become(from, to)])]
    #[no_panic]
    pub fn can_become(&self, to: Status) -> bool {
        !matches!((self, to), (Status::Done, Status::Doing))
    }

    #[spec(fn(&Status[@s]) -> bool[s == Status::Done])]
    #[no_panic]
    pub fn is_done(&self) -> bool {
        matches!(self, Status::Done)
    }

    /// Inverse of `name`.
    pub fn named(name: &str) -> Option<Status> {
        Status::ALL.into_iter().find(|status| status.name() == name)
    }

    #[no_panic]
    pub fn name(&self) -> &'static str {
        match self {
            Status::Open => "open",
            Status::Doing => "doing",
            Status::Done => "done",
        }
    }

    /// How the status starts a line of the text format.
    #[no_panic]
    fn mark(&self) -> &'static str {
        match self {
            Status::Open => "[ ] ",
            Status::Doing => "[~] ",
            Status::Done => "[x] ",
        }
    }
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
#[no_panic]
pub fn priority(n: u8) -> Option<Priority> {
    if HIGHEST_PRIORITY <= n && n <= LOWEST_PRIORITY {
        Some(n)
    } else {
        None
    }
}

/// A todo's title: one line with something on it.
///
/// Flux cannot look inside a string, so the struct is opaque and its
/// functions are trusted: this is the one place the claim "not blank" is
/// made by hand. Everything outside is checked against it as usual.
#[opaque]
#[refined_by(chars: int)]
#[invariant(chars > 0)]
pub struct Title(String);

impl Title {
    /// Makes a title from any text, or `None` when nothing is left after
    /// putting it on one line and trimming it.
    #[trusted]
    #[spec(fn(&str) -> Option<Title>)]
    pub fn new(text: &str) -> Option<Title> {
        let line = text.replace(['\n', '\r'], " ");
        let line = line.trim();
        if line.is_empty() {
            None
        } else {
            Some(Title(line.to_string()))
        }
    }

    /// Only the specs need the length, so only Flux compiles this.
    #[cfg(flux)]
    #[trusted]
    #[spec(fn(&Title[@t]) -> usize[t.chars])]
    pub fn chars(&self) -> usize {
        self.0.chars().count()
    }

    #[trusted]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Where a priority's count sits in the array `open_by_priority` returns.
#[spec(fn(p: Priority) -> usize{slot: slot < PRIORITY_LEVELS})]
#[no_panic]
pub fn slot(priority: Priority) -> usize {
    (priority - HIGHEST_PRIORITY) as usize
}

#[refined_by(status: Status)]
pub struct Todo {
    pub title: Title,
    #[field(Status[status])]
    pub status: Status,
    #[field(Priority)]
    pub priority: Priority,
}

impl Todo {
    /// The todo as one line of the text format, e.g. `[x] !2 title`.
    pub fn line(&self) -> String {
        let mark = self.status.mark();
        format!(
            "{mark}{PRIORITY_TAG}{} {}",
            self.priority,
            self.title.as_str()
        )
    }

    /// Inverse of `line`. A missing mark means open and a missing or invalid
    /// priority means the default, so any hand-edited file still loads. A
    /// line with no title on it is not a todo.
    fn from_line(line: &str) -> Option<Todo> {
        let mut status = Status::Open;
        let mut rest = line;
        for candidate in Status::ALL {
            if let Some(after_mark) = line.strip_prefix(candidate.mark()) {
                status = candidate;
                rest = after_mark;
            }
        }
        let (priority, title) = split_priority(rest);
        Some(Todo {
            title: Title::new(title)?,
            status,
            priority,
        })
    }

    /// Moves the todo to another status if the rules allow it, and says
    /// whether it did. Flux checks this against the rules: it moves exactly
    /// when `can_become` holds and otherwise leaves the status alone.
    #[spec(fn(self: &mut Todo[@t], to: Status) -> bool[can_become(t.status, to)]
           ensures self: Todo{v: v.status == if can_become(t.status, to) { to } else { t.status }})]
    #[no_panic]
    pub fn try_become(&mut self, to: Status) -> bool {
        let allowed = self.status.can_become(to);
        if allowed {
            self.status = to;
        }
        allowed
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
    #[no_panic]
    pub fn left(&self) -> usize {
        self.total - self.done
    }

    /// The done share as a whole percentage. There is no share of nothing,
    /// so the caller must have at least one todo.
    #[spec(fn(&Progress[@p]) -> usize{v: v <= 100} requires p.total > 0)]
    #[no_panic]
    pub fn percent(&self) -> usize {
        // In u128 the product cannot overflow, however long the list is.
        (self.done as u128 * 100 / self.total as u128) as usize
    }
}

#[refined_by(len: int)]
#[invariant(len >= 0)]
pub struct TodoList {
    #[field(Vec<Todo>[len])]
    items: Vec<Todo>,
}

impl TodoList {
    #[spec(fn() -> TodoList[0])]
    #[no_panic]
    pub fn new() -> TodoList {
        TodoList { items: Vec::new() }
    }

    /// Reads the text format: one todo per line, lines without a title ignored.
    pub fn parse(text: &str) -> TodoList {
        let mut list = TodoList::new();
        for line in text.lines() {
            if let Some(todo) = Todo::from_line(line) {
                list.push(todo);
            }
        }
        list
    }

    /// Writes the text format that `parse` reads.
    pub fn to_text(&self) -> String {
        let mut text = String::new();
        self.each_index(|index| {
            text.push_str(&self[index].line());
            text.push('\n');
        });
        text
    }

    /// Calls `visit` with every index of the list, in order. The closure's
    /// signature promises it an index that is valid for this list, so it can
    /// hand the index straight back to `get`.
    #[spec(fn(&TodoList[@n], visit: F) where F: FnMut(usize{i: i < n}))]
    pub fn each_index<F>(&self, mut visit: F)
    where
        F: FnMut(usize),
    {
        for i in 0..self.len() {
            visit(i);
        }
    }

    #[spec(fn(&TodoList[@n]) -> usize[n])]
    #[no_panic]
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Appends an open todo.
    #[spec(fn(self: &mut TodoList[@n], title: Title) ensures self: TodoList[n + 1])]
    pub fn add(&mut self, title: Title) {
        self.push(Todo {
            title,
            status: Status::Open,
            priority: DEFAULT_PRIORITY,
        });
    }

    /// Turns the 1-based position a user types into its index: `Some` exactly
    /// when a todo sits at that position, and then the index just before it.
    #[spec(fn(&TodoList[@n], pos: usize) -> Option<usize[pos - 1]>[is_position(pos, n)])]
    #[no_panic]
    pub fn locate(&self, pos: usize) -> Option<usize> {
        if 1 <= pos && pos <= self.len() {
            Some(pos - 1)
        } else {
            None
        }
    }

    /// Moves a todo to another status if the rules allow it, and says
    /// whether it did.
    #[spec(fn(&mut TodoList[@n], usize{i: i < n}, to: Status) -> bool)]
    pub fn set_status(&mut self, index: usize, to: Status) -> bool {
        self.items[index].try_become(to)
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
        for i in 0..total {
            invariant!(done: int; done <= i);
            if self[i].status.is_done() {
                done += 1;
            }
        }
        Progress { done, total }
    }

    /// How many open todos there are at each priority, highest first. A
    /// priority always has a slot: its range is the array's length.
    // No count can pass the list's length, but that needs a fact about every
    // element of the array on every turn of the loop, which Flux cannot infer.
    #[opts(check_overflow = "none")]
    pub fn open_by_priority(&self) -> [usize; PRIORITY_LEVELS] {
        let mut counts = [0; PRIORITY_LEVELS];
        self.each_index(|index| {
            let todo = &self[index];
            if !todo.status.is_done() {
                counts[slot(todo.priority)] += 1;
            }
        });
        counts
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

/// `list[index]`, for an index Flux can prove is in the list. `in_bounds` is
/// the associated refinement the `Index` trait asks every implementation to
/// define: it is what makes the proof work for the indexing syntax.
#[assoc(fn in_bounds(list: TodoList, index: int) -> bool { index < list.len })]
impl Index<usize> for TodoList {
    type Output = Todo;

    #[spec(fn(&TodoList[@list], index: usize{<TodoList as Index<usize>>::in_bounds(list, index)}) -> &Todo)]
    fn index(&self, index: usize) -> &Todo {
        &self.items[index]
    }
}
