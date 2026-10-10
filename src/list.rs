//! The todo list and its on-disk text format.
//!
//! Flux tracks the list's length in its type, so every index handed to
//! indexing, `set_status` or `move_to` is proved in bounds at compile time. The
//! only way to obtain such an index from untrusted input is `locate`.

use std::{cmp::min, collections::BTreeSet, ops::Index};

use flux_rs::{attrs::*, bitvec::BV32, defs, macros::qualifier};

const PRIORITY_TAG: char = '!';

pub const HIGHEST_PRIORITY: u8 = 1;
pub const LOWEST_PRIORITY: u8 = 3;
const DEFAULT_PRIORITY: Priority = 2;
pub const PRIORITY_LEVELS: usize = (LOWEST_PRIORITY - HIGHEST_PRIORITY + 1) as usize;

/// Every bit that stands for a status, and no other. The attribute gives
/// Flux the value, which it cannot read out of a constant of this type.
#[constant(bv_int_to_bv32(7))]
const STATUS_BITS: BV32 = BV32::new(7);

// The rules of this module, named once and used by the specs below.
defs! {
    fn is_priority(p: int) -> bool { HIGHEST_PRIORITY <= p && p <= LOWEST_PRIORITY }
    fn is_position(pos: int, len: int) -> bool { 1 <= pos && pos <= len }
    // The shortest list that is `need` long and also has `index` in it.
    fn need_with(need: int, index: int) -> int {
        let past = index + 1;
        if past > need { past } else { need }
    }
    // How each status starts a line of the text format.
    fn mark_of(status: Status) -> str {
        if status == Status::Open { "[ ] " } else if status == Status::Doing { "[~] " } else { "[x] " }
    }
    // Every slot of an array with one slot per priority belongs to a
    // priority: the number of slots and the range of priorities agree.
    // Flux wants the bound written as a number; the theorem that uses this
    // also checks that number against `PRIORITY_LEVELS`.
    fn every_slot_has_a_priority() -> bool {
        forall slot in 0 .. 3 { is_priority(slot + HIGHEST_PRIORITY) }
    }
    // The number a digit character stands for. `cast` turns the character
    // into its code so the spec can do arithmetic on it.
    fn digit_value(c: char) -> int { cast(c) - cast('0') }
    // Whether nothing is left of a text once it is trimmed. It has no body:
    // Flux cannot look inside a string, so all it knows is that this is a
    // function, giving the same answer for the same text every time.
    fn blank(text: str) -> bool;
    // Where each status sits in `Status::ALL` and in the per-status counts.
    fn slot_of(status: Status) -> int {
        if status == Status::Open { 0 } else if status == Status::Doing { 1 } else { 2 }
    }
    // The bit that stands for each status in a `StatusSet`, and what makes a
    // bit pattern a set of statuses: no bit outside those three.
    fn bit_of(status: Status) -> bitvec<32> {
        if status == Status::Open { 1 } else if status == Status::Doing { 2 } else { 4 }
    }
    // Flux does not work out `<<` on integers. This tells it the three
    // facts `Status::bit` needs: where a one lands when shifted to a slot.
    property OneShiftedToASlot[<<](x, y) {
        (x == 1 && y == 0 => [<<](x, y) == 1)
            && (x == 1 && y == 1 => [<<](x, y) == 2)
            && (x == 1 && y == 2 => [<<](x, y) == 4)
    }
    fn only_status_bits(bits: bitvec<32>) -> bool { bits & STATUS_BITS == bits }
    // A finished todo must be reopened before it can be worked on again.
    // Hidden: other code may rely on the rule holding, not on what it says,
    // so the rule can change without any other proof having to.
    #[hide]
    fn can_become(from: Status, to: Status) -> bool { !(from == Status::Done && to == Status::Doing) }
}

/// Where a todo stands. `reflect` lets the specs talk about the variants.
#[reflect]
#[derive(Clone, Copy, PartialEq)]
#[trusted_derive(reason = "Flux cannot check the discriminant comparison derive writes")]
pub enum Status {
    Open,
    Doing,
    Done,
}

// `==` on statuses is written by `derive`, so there is no function to put a
// spec on. A detached spec attaches one from outside: `==` is true exactly
// for equal statuses. The derived body is trusted to do that.
#[specs {
    impl std::cmp::PartialEq for Status {
        fn eq(&Status[@a], &Status[@b]) -> bool[a == b];
    }
}]
const _: () = ();

pub const STATUSES: usize = 3;

impl Status {
    const ALL: [Status; STATUSES] = [Status::Open, Status::Doing, Status::Done];

    /// Where this status sits in `ALL` and in `TodoList::by_status`.
    #[no_panic]
    #[spec(fn(&Status[@status]) -> usize[slot_of(status)])]
    pub fn slot(&self) -> usize {
        match self {
            Status::Open => 0,
            Status::Doing => 1,
            Status::Done => 2,
        }
    }

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

    /// The bit that stands for this status in a `StatusSet`: the one at
    /// its slot, so the bits follow the slots and are not listed again.
    #[spec(fn(&Status[@status]) -> BV32[bit_of(status)])]
    fn bit(&self) -> BV32 {
        BV32::new(1 << self.slot())
    }

    /// How the status starts a line of the text format.
    #[no_panic]
    #[spec(fn(&Status[@status]) -> &str[mark_of(status)])]
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

/// Reads the one character that stands for a priority in the text format:
/// `Some` exactly when it is the digit of a priority, and then that number.
#[spec(fn(char[@c]) -> Option<u8[digit_value(c)]>[is_priority(digit_value(c))])]
fn priority_digit(c: char) -> Option<Priority> {
    match c.to_digit(10) {
        Some(n) => priority(n as u8),
        None => None,
    }
}

/// What std's `char::to_digit` does for decimal digits, said in terms of
/// the character's code. Only Flux reads this.
#[cfg(flux)]
mod char_specs {
    use flux_rs::attrs::*;

    defs! {
        use super::digit_value;
    }

    #[extern_spec]
    impl char {
        #[spec(fn(char[@c], u32[10]) -> Option<u32[digit_value(c)]>['0' <= c && c <= '9'])]
        fn to_digit(self, radix: u32) -> Option<u32>;
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
    /// putting it on one line and trimming it: when the text is `blank`.
    #[trusted]
    #[spec(fn(&str[@text]) -> Option<Title>[!blank(text)])]
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

/// What Flux can prove about titles knowing only that `blank` is a function.
/// Only Flux compiles these.
#[cfg(flux)]
mod title_theorems {
    #![allow(dead_code)]

    use flux_rs::attrs::*;

    use super::Title;

    #[spec(fn(&str) -> bool[true])]
    fn text_that_made_a_title_makes_one_again(text: &str) -> bool {
        Title::new(text).is_some() == Title::new(text).is_some()
    }
}

/// A set of statuses, kept as one bit per status. Flux reasons about the
/// bits themselves: a set never holds a bit that is not a status.
#[derive(Clone, Copy)]
#[refined_by(bits: bitvec<32>)]
#[invariant(only_status_bits(bits))]
pub struct StatusSet {
    #[field({BV32[bits] | only_status_bits(bits)})]
    bits: BV32,
}

impl StatusSet {
    #[spec(fn() -> StatusSet[0])]
    pub fn none() -> StatusSet {
        StatusSet { bits: BV32::new(0) }
    }

    #[spec(fn(StatusSet[@bits], Status[@status]) -> StatusSet[bits | bit_of(status)])]
    pub fn with(self, status: Status) -> StatusSet {
        StatusSet {
            bits: self.bits | status.bit(),
        }
    }

    #[spec(fn(&StatusSet[@bits], Status[@status]) -> bool[bits & bit_of(status) != 0])]
    pub fn contains(&self, status: Status) -> bool {
        self.bits & status.bit() != 0
    }

    #[spec(fn() -> StatusSet[STATUS_BITS])]
    pub fn all() -> StatusSet {
        StatusSet { bits: STATUS_BITS }
    }
}

/// What makes `StatusSet` behave like a set, proved from its bits for every
/// set and every status. Only Flux compiles these.
#[cfg(flux)]
mod status_set_theorems {
    #![allow(dead_code)]

    use flux_rs::attrs::*;

    use super::{Status, StatusSet};

    #[spec(fn(Status) -> bool[false])]
    fn the_empty_set_holds_nothing(status: Status) -> bool {
        StatusSet::none().contains(status)
    }

    #[spec(fn(Status) -> bool[true])]
    fn the_full_set_holds_every_status(status: Status) -> bool {
        StatusSet::all().contains(status)
    }

    #[spec(fn(StatusSet, Status) -> bool[true])]
    fn a_status_put_in_is_found(set: StatusSet, status: Status) -> bool {
        set.with(status).contains(status)
    }

    #[spec(fn(StatusSet[@bits], Status[@added], Status[@other]) -> bool[true] requires added != other)]
    fn putting_one_status_in_adds_no_other(set: StatusSet, added: Status, other: Status) -> bool {
        set.with(added).contains(other) == set.contains(other)
    }
}

/// What std's `min` keeps. The spec is generic in what is known about the
/// two values: whatever property `p` both have, the result has too, so the
/// smaller of two priorities is a priority and nothing here mentions
/// priorities. Only Flux reads this.
#[cfg(flux)]
mod cmp_specs {
    use flux_rs::attrs::*;

    #[extern_spec(std::cmp)]
    #[spec(fn[hrn p: T -> bool](T{v: p(v)}, T{v: p(v)}) -> T{v: p(v)})]
    fn min<T: Ord>(a: T, b: T) -> T;
}

/// Every priority, most urgent first. Whatever the iterator is made of, each
/// item it yields is a proved priority.
#[spec(fn() -> impl Iterator<Item = Priority>)]
pub fn priorities() -> impl Iterator<Item = Priority> {
    // The length ties this list to the range: change one and not the other
    // and it stops compiling.
    let all: [Priority; PRIORITY_LEVELS] = [1, 2, 3];
    all.into_iter()
}

/// Theorems about the status rules, proved once for every status rather than
/// tested for some. Only Flux compiles them.
#[cfg(flux)]
mod status_theorems {
    #![allow(dead_code)]

    use flux_rs::{attrs::*, defs};

    // The specs name the variants; Rust itself sees no use of the type.
    use super::PRIORITY_LEVELS;
    #[allow(unused_imports)]
    use super::Status;

    defs! {
        use super::can_become;
        use super::mark_of;
        use super::every_slot_has_a_priority;
    }

    /// `PRIORITY_LEVELS` slots are exactly enough: none is left without a
    /// priority to count.
    #[trusted(reason = "a statement, not code: whoever calls it must prove it")]
    #[spec(fn(usize[@levels]) requires levels == 3 && every_slot_has_a_priority())]
    fn no_priority_slot_is_spare(_levels: usize) {}

    /// Whatever its status, a todo can always be reopened and always be
    /// finished: no status is a dead end.
    #[trusted(reason = "a statement, not code: whoever calls it must prove it")]
    #[spec(fn() requires forall s. can_become(s, Status::Open) && can_become(s, Status::Done))]
    fn no_status_is_a_dead_end() {}

    /// Two statuses never share a mark, so the mark at the start of a line
    /// in the file says which status it is.
    #[trusted(reason = "a statement, not code: whoever calls it must prove it")]
    #[spec(fn() requires forall s.
        (mark_of(s) == mark_of(Status::Open) => s == Status::Open)
        && (mark_of(s) == mark_of(Status::Doing) => s == Status::Doing)
        && (mark_of(s) == mark_of(Status::Done) => s == Status::Done))]
    fn marks_tell_statuses_apart() {}

    #[reveal(can_become)]
    fn proofs() {
        no_status_is_a_dead_end();
        marks_tell_statuses_apart();
        no_priority_slot_is_spare(PRIORITY_LEVELS);
    }
}

/// Where a priority's count sits in the array `open_by_priority` returns.
#[spec(fn(p: Priority) -> usize{slot: slot < PRIORITY_LEVELS})]
#[no_panic]
pub fn slot(priority: Priority) -> usize {
    (priority - HIGHEST_PRIORITY) as usize
}

/// One todo. Its status and its priority are both in its type, so a
/// function that changes one can say it leaves the other as it was.
#[refined_by(status: Status, priority: int)]
pub struct Todo {
    pub title: Title,
    #[field(Status[status])]
    pub status: Status,
    #[field({u8[priority] | is_priority(priority)})]
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

    /// The slot of this todo in `TodoList::open_by_priority`: its priority's,
    /// unless it is done and so not counted.
    #[spec(fn(&Todo) -> Option<usize{slot: slot < PRIORITY_LEVELS}>)]
    fn open_priority_slot(&self) -> Option<usize> {
        if self.status.is_done() {
            None
        } else {
            Some(slot(self.priority))
        }
    }

    /// The slot of this todo in `TodoList::by_status`.
    #[spec(fn(&Todo) -> Option<usize{slot: slot < STATUSES}>)]
    fn status_slot(&self) -> Option<usize> {
        Some(self.status.slot())
    }

    /// Moves the todo to another status if the rules allow it, and says
    /// whether it did. Flux checks this against the rules: it moves exactly
    /// when `can_become` holds and otherwise leaves the status alone. Either
    /// way the rest of the todo, its priority, is what it was.
    #[spec(fn(self: &mut Todo[@t], to: Status) -> bool[can_become(t.status, to)]
           ensures self: Todo[Todo { status: if can_become(t.status, to) { to } else { t.status }, ..t }])]
    #[no_panic]
    pub fn try_become(&mut self, to: Status) -> bool {
        let allowed = self.status.can_become(to);
        if allowed {
            self.status = to;
        }
        allowed
    }

    /// Changes the priority and nothing else: the status is what it was.
    #[spec(fn(self: &mut Todo[@t], Priority[@priority]) ensures self: Todo[Todo { priority: priority, ..t }])]
    #[no_panic]
    pub fn set_priority(&mut self, priority: Priority) {
        self.priority = priority;
    }
}

/// Splits `!2 title` into its priority and title. Text without a valid
/// priority tag is all title.
#[spec(fn(_) -> (Priority, _))]
fn split_priority(rest: &str) -> (Priority, &str) {
    if let Some((tag, title)) = rest.split_once(' ')
        && let Some(digit) = tag.strip_prefix(PRIORITY_TAG)
        && let Ok(c) = digit.parse()
        && let Some(priority) = priority_digit(c)
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

/// The todos picked for one command, by index. Flux tracks the set of
/// indices itself, not just how many there are, so it knows which index is
/// in and which is not; `need` is the shortest list all of them are in.
#[opaque]
#[refined_by(picked: Set<int>, need: int)]
#[invariant(need >= 0)]
pub struct Picks(BTreeSet<usize>);

impl Picks {
    #[trusted]
    #[spec(fn() -> Picks[set_empty(0), 0])]
    pub fn none() -> Picks {
        Picks(BTreeSet::new())
    }

    /// Adds an index, and says whether it was new.
    #[trusted]
    #[spec(fn(self: &mut Picks[@p], usize[@index]) -> bool[!set_is_in(index, p.picked)]
           ensures self: Picks[set_union(set_singleton(index), p.picked), need_with(p.need, index)])]
    pub fn pick(&mut self, index: usize) -> bool {
        self.0.insert(index)
    }

    #[trusted]
    #[spec(fn(&Picks[@p], usize[@index]) -> bool[set_is_in(index, p.picked)])]
    pub fn contains(&self, index: usize) -> bool {
        self.0.contains(&index)
    }
}

/// What the set of picked indices lets Flux prove about any picks at all.
/// Only Flux compiles these.
#[cfg(flux)]
mod picks_theorems {
    #![allow(dead_code)]

    use flux_rs::attrs::*;

    use super::Picks;

    #[spec(fn(Picks, usize) -> bool[false])]
    fn an_index_cannot_be_picked_twice(mut picks: Picks, index: usize) -> bool {
        picks.pick(index);
        picks.pick(index)
    }

    #[spec(fn(Picks, usize[@index], usize{other: other != index}) -> bool[true])]
    fn picking_one_index_picks_no_other(mut picks: Picks, index: usize, other: usize) -> bool {
        let before = picks.contains(other);
        picks.pick(index);
        picks.contains(other) == before
    }
}

/// A choice of todos out of a list, made by number or by what they are.
/// `need` is the shortest list the choice can be made from. Unless an
/// implementation says otherwise that is no length at all: a choice by what
/// the todos are fits any list.
#[assoc(fn need(selection: Self) -> int { 0 })]
pub trait Selection {
    fn has(&self, index: usize, todo: &Todo) -> bool;
}

impl Selection for StatusSet {
    fn has(&self, _index: usize, todo: &Todo) -> bool {
        self.contains(todo.status)
    }
}

#[assoc(fn need(picks: Picks) -> int { picks.need })]
impl Selection for Picks {
    fn has(&self, index: usize, _todo: &Todo) -> bool {
        self.contains(index)
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
        self.items[index].set_priority(priority);
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
            if self[i].status.is_done() {
                done += 1;
            }
        }
        Progress { done, total }
    }

    /// Inverse of `locate`: the number a user sees for an index.
    #[spec(fn(&TodoList[@n], index: usize{index < n}) -> usize[index + 1])]
    pub fn position(&self, index: usize) -> usize {
        // The length is a usize, so it is what tells Flux that an index
        // below it can be increased without overflow.
        let _fits_in_usize = self.len();
        index + 1
    }

    /// The todos as a slice whose length is the list's, so that walking it
    /// with `enumerate` yields indices proved to be in the list.
    #[spec(fn(&TodoList[@n]) -> &[Todo][n])]
    fn todos(&self) -> &[Todo] {
        &self.items
    }

    /// The todo to work on next: the first one not done among those of the
    /// most urgent priority that has any, or `None` when everything is done.
    #[spec(fn(&TodoList[@n]) -> Option<usize{i: i < n}>)]
    pub fn next(&self) -> Option<usize> {
        let mut most_urgent: Option<Priority> = None;
        for todo in self.todos().iter() {
            if !todo.status.is_done() {
                most_urgent = Some(match most_urgent {
                    Some(priority) => min(priority, todo.priority),
                    None => todo.priority,
                });
            }
        }
        let wanted = most_urgent?;
        for (i, todo) in self.todos().iter().enumerate() {
            if !todo.status.is_done() && todo.priority == wanted {
                return Some(i);
            }
        }
        None
    }

    /// How many open todos there are at each priority, highest first.
    pub fn open_by_priority(&self) -> [usize; PRIORITY_LEVELS] {
        self.tally(Todo::open_priority_slot)
    }

    /// How many todos there are in each status, in the order of `Status::ALL`.
    pub fn by_status(&self) -> [usize; STATUSES] {
        self.tally(Todo::status_slot)
    }

    /// Counts todos into `N` slots: `slot_of` says which slot a todo goes
    /// to, or `None` to leave it out. It works for any number of slots
    /// because the type of the function passed in is tied to the array's
    /// length.
    // No count can pass the list's length, but that needs a fact about every
    // element of the array on every turn of the loop, which Flux cannot infer.
    #[opts(check_overflow = "none")]
    #[spec(fn(&TodoList, slot_of: fn(&Todo) -> Option<usize{slot: slot < N}>) -> [usize; N])]
    fn tally<const N: usize>(&self, slot_of: fn(&Todo) -> Option<usize>) -> [usize; N] {
        let mut counts = [0; N];
        for i in 0..self.len() {
            if let Some(slot) = slot_of(&self[i]) {
                counts[slot] += 1;
            }
        }
        counts
    }

    /// Takes the chosen todos out of the list and returns them in the order
    /// they had. The list shrinks by exactly as many todos as come back:
    /// none is lost and none is taken twice. Walking from the end keeps every index still to visit valid
    /// while the list shrinks under it.
    #[spec(fn(self: &mut TodoList[@n], &S{s: <S as Selection>::need(s) <= n}) -> Vec<Todo>[#taken] ensures self: TodoList[n - taken])]
    pub fn remove_all<S: Selection>(&mut self, chosen: &S) -> Vec<Todo> {
        let mut removed = Vec::new();
        let mut index = self.len();
        while index > 0 {
            // What stays true on every turn, which Flux does not find alone:
            // the todos left and the todos taken add up to the original list.
            qualifier!(left: int, taken: int, all: int; left + taken == all);
            index -= 1;
            if chosen.has(index, &self.items[index]) {
                removed.push(self.items.remove(index));
            }
        }
        removed.reverse();
        removed
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
