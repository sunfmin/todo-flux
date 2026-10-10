//! Misuses of this crate that Flux must reject.
//!
//! Every function here is wrong on purpose. `should_fail` turns that around:
//! verification fails if Flux ever *accepts* one, so a spec cannot be
//! weakened without `./verify.sh` noticing. Nothing here is compiled into the
//! program.
#![allow(dead_code)]

use flux_rs::attrs::*;

use crate::list::TodoList;

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
fn priority_taken_from_raw_number(list: &mut TodoList, n: u8) {
    if let Some(index) = list.locate(1) {
        list.set_priority(index, n);
    }
}

#[should_fail]
#[spec(fn(&TodoList[@n]) -> usize[n])]
fn done_count_mistaken_for_length(list: &TodoList) -> usize {
    list.done_count()
}

#[should_fail]
fn percent_of_empty_list() -> usize {
    crate::percent(0, 0)
}

#[should_fail]
fn percent_of_more_than_everything() -> usize {
    crate::percent(3, 2)
}
