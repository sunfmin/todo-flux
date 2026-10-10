//! Misuses of this crate that Flux must reject.
//!
//! Every function here is wrong on purpose. `should_fail` turns that around:
//! verification fails if Flux ever *accepts* one, so a spec cannot be
//! weakened without `./verify.sh` noticing. Nothing here is compiled into the
//! program.
#![allow(dead_code)]

use flux_rs::{attrs::*, defs};

use crate::{
    Command,
    list::{Progress, Status, StatusSet, Title, Todo, TodoList},
};

defs! {
    use crate::list::can_become;
}

#[should_fail]
fn position_used_without_locate(list: &mut TodoList, pos: usize) {
    list.set_status(pos, Status::Done);
}

#[should_fail]
fn index_reused_after_remove(mut list: TodoList, pos: usize) {
    if let Some(index) = list.locate(pos) {
        list.remove(index);
        let _ = &list[index];
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

#[should_fail]
fn priority_used_as_slot_without_offset(list: &TodoList) -> usize {
    match list.locate(1) {
        Some(index) => list.open_by_priority()[list[index].priority as usize],
        None => 0,
    }
}

#[should_fail]
#[spec(fn(todo: &mut Todo[@t], to: Status) -> bool[can_become(t.status, to)]
       ensures todo: Todo{v: v.status == if can_become(t.status, to) { to } else { t.status }})]
fn status_changed_without_asking_the_rules(todo: &mut Todo, to: Status) -> bool {
    todo.status = to;
    todo.status.can_become(to)
}

#[should_fail]
#[spec(fn(&TodoList[@n], visit: F) where F: FnMut(usize{i: i < n}))]
fn visitor_handed_an_index_past_the_end<F>(list: &TodoList, mut visit: F)
where
    F: FnMut(usize),
{
    visit(list.len());
}

#[should_fail]
#[spec(fn(&Title) -> usize[0])]
fn title_with_nothing_on_it(title: &Title) -> usize {
    title.chars()
}

#[should_fail]
fn percent_multiplied_in_usize(progress: &Progress) -> usize {
    progress.done * 100
}

#[should_fail]
fn command_run_on_a_list_it_was_not_checked_against(list: &mut TodoList, command: Command) {
    if let Command::Remove(index) = command {
        list.set_status(index, Status::Done);
    }
}

#[should_fail]
#[spec(fn(&TodoList[@n], usize{i: i <= n}) -> Command{c: c.need <= n})]
fn command_for_one_past_the_end(_list: &TodoList, index: usize) -> Command {
    Command::Remove(index)
}

#[should_fail]
#[no_panic]
#[spec(fn(&TodoList[@n], usize{i: i < n}) -> bool)]
fn vec_access_claimed_not_to_panic(list: &TodoList, index: usize) -> bool {
    list[index].status.is_done()
}

#[should_fail]
#[spec(fn(Status) -> bool[false])]
fn a_status_reported_unequal_to_itself(status: Status) -> bool {
    status == status
}

#[should_fail]
#[spec(fn(&Todo) -> Option<usize{slot: slot < 2}>)]
fn status_slot_claimed_to_fit_two_slots(todo: &Todo) -> Option<usize> {
    Some(todo.status.slot())
}

#[should_fail]
#[spec(fn[hrn p: int -> bool](a: u8{p(a)}, b: u8{p(b)}) -> u8{v: p(v)})]
fn sum_claimed_to_keep_any_property(a: u8, b: u8) -> u8 {
    a / 2 + b / 2
}

#[should_fail]
#[spec(fn(StatusSet, Status) -> bool[true])]
fn a_status_found_in_a_set_it_was_never_put_in(set: StatusSet, status: Status) -> bool {
    set.contains(status)
}
