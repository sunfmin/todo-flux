//! Misuses of this crate that Flux must reject.
//!
//! Every function here is wrong on purpose. `should_fail` turns that around:
//! verification fails if Flux ever *accepts* one, so a spec cannot be
//! weakened without `./verify.sh` noticing. Nothing here is compiled into the
//! program.
#![allow(dead_code)]

use flux_rs::attrs::*;

use crate::list::{Progress, TodoList};

#[should_fail]
fn position_used_without_locate(list: &mut TodoList, pos: usize) {
    list.set_done(pos, true);
}

#[should_fail]
fn index_reused_after_remove(mut list: TodoList, pos: usize) {
    if let Some(index) = list.locate(pos) {
        list.remove(index);
        list.get(index);
    }
}

#[should_fail]
#[spec(fn(&TodoList[@n], pos: usize{1 <= pos && pos <= n}) -> usize[pos])]
fn locate_mistaken_for_identity(list: &TodoList, pos: usize) -> usize {
    list.locate(pos).unwrap()
}

#[should_fail]
fn priority_taken_from_raw_number(list: &mut TodoList, n: u8) {
    if let Some(index) = list.locate(1) {
        list.set_priority(index, n);
    }
}

#[should_fail]
#[spec(fn(&TodoList[@n]) -> usize[n])]
fn done_mistaken_for_length(list: &TodoList) -> usize {
    list.progress().done
}

#[should_fail]
fn percent_of_empty_list() -> usize {
    TodoList::new().progress().percent()
}

#[should_fail]
fn more_done_than_there_are() -> Progress {
    Progress { done: 3, total: 2 }
}
