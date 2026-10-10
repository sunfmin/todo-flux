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
    list::{Picks, Priority, Progress, Selection, Status, StatusSet, Title, Todo, TodoList},
};

defs! {
    use crate::list::can_become;
}

#[should_fail]
fn position_used_without_locate(list: &mut TodoList, pos: usize) {
    list.set_status(pos, Status::Done);
}

#[should_fail]
#[spec(fn(TodoList[@n], usize, &Picks{p: p.need <= n}))]
fn index_reused_after_removing(mut list: TodoList, pos: usize, picks: &Picks) {
    if let Some(index) = list.locate(pos) {
        list.remove_all(picks);
        let _ = &list[index];
    }
}

#[should_fail]
fn picks_run_on_a_list_they_were_not_checked_against(list: &mut TodoList, picks: &Picks) {
    list.remove_all(picks);
}

#[should_fail]
#[spec(fn(Picks, usize) -> bool[true])]
fn an_index_claimed_new_to_any_picks(mut picks: Picks, index: usize) -> bool {
    picks.pick(index)
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
    if let Command::SetStatus(index, to) = command {
        list.set_status(index, to);
    }
}

#[should_fail]
#[spec(fn(&TodoList[@n], usize{i: i <= n}) -> Command{c: c.need <= n})]
fn command_for_one_past_the_end(_list: &TodoList, index: usize) -> Command {
    Command::SetStatus(index, Status::Done)
}

#[should_fail]
#[no_panic]
#[spec(fn(&Vec<Todo>[@n], usize{i: i < n}) -> bool)]
fn std_vec_access_claimed_not_to_panic(todos: &Vec<Todo>, index: usize) -> bool {
    todos[index].status.is_done()
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

#[should_fail]
#[spec(fn(&str) -> bool[true])]
fn any_text_claimed_to_make_a_title(text: &str) -> bool {
    Title::new(text).is_some()
}

#[should_fail]
#[spec(fn(&str, &str) -> bool[true])]
fn two_texts_claimed_to_be_blank_together(a: &str, b: &str) -> bool {
    Title::new(a).is_some() == Title::new(b).is_some()
}

#[should_fail]
#[spec(fn(todo: &mut Todo[@t], Priority[@priority]) ensures todo: Todo[Todo { priority: priority, ..t }])]
fn priority_change_that_also_reopens_the_todo(todo: &mut Todo, priority: Priority) {
    todo.priority = priority;
    todo.status = Status::Open;
}

#[should_fail]
#[spec(fn(&Command[@c]) -> bool[c.need > 0])]
fn every_command_claimed_to_need_a_todo(_command: &Command) -> bool {
    true
}

#[should_fail]
#[spec(fn(list: &mut TodoList[@n], &Picks{p: p.need <= n}) ensures list: TodoList[n])]
fn removal_claimed_to_keep_the_list_whole(list: &mut TodoList, picks: &Picks) {
    list.remove_all(picks);
}

#[should_fail]
#[spec(fn(Priority, u8) -> Priority)]
fn smaller_of_a_priority_and_any_number_claimed_a_priority(priority: Priority, n: u8) -> Priority {
    std::cmp::min(priority, n)
}

#[should_fail]
fn index_from_walking_one_slice_used_on_another(walked: &[Todo], other: &[Todo]) {
    for (index, _) in walked.iter().enumerate() {
        let _ = &other[index];
    }
}

#[should_fail]
#[spec(fn(&S, S::By) -> bool[true])]
fn any_selection_claimed_to_hold_everything<S: Selection>(selection: &S, by: S::By) -> bool {
    selection.has(by)
}

// The same arithmetic as `Progress::percent`, which Flux accepts. Here the
// solver is told to forget what `*` and `/` compute, and the bound is gone:
// that bound rests on the arithmetic itself, not on anything weaker.
#[should_fail]
#[opts(uif_ops = "*,/")]
#[spec(fn(&Progress[@p]) -> usize{v: v <= 100} requires p.total > 0)]
fn percent_bound_claimed_without_what_times_and_divide_compute(progress: &Progress) -> usize {
    (progress.done as u128 * 100 / progress.total as u128) as usize
}
