//! `todo` — a command-line todo list whose rules are verified by Flux.

#![cfg_attr(flux, feature(allocator_ext))]

// Linking these crates is what brings the std refinement specs into scope.
extern crate flux_alloc;
extern crate flux_core;

mod list;
#[cfg(flux)]
mod rejected;
mod rvec;

use std::{
    cmp::max,
    env, fs,
    io::{self, Write},
    path::PathBuf,
    process::ExitCode,
};

use flux_rs::{attrs::*, defs, macros::invariant};
use list::{
    Choice, HIGHEST_PRIORITY, InProgress, LOWEST_PRIORITY, MostUrgent, Picks, Priority, Removed,
    Status, StatusSet, Title, TodoList, priorities, priority, slot,
};

const USAGE: &str = "\
usage: todo [list]          show all todos
       todo list <status>.. show only open, doing or done todos
       todo next            show the most urgent todo that is not done
       todo now             show the todo being worked on
       todo add <title>     add a todo
       todo start <n>       mark todo n as being worked on
       todo done <n>        mark todo n as done
       todo undo <n>        mark todo n as open again
       todo pri <n> <1-3>   set the priority of todo n (1 is highest)
       todo mv <n> <m>      move todo n to position m
       todo rm <n|n-m>..    delete the todos numbered; 2-4 means 2, 3 and 4
       todo clear           delete every done todo
       todo count           show how many todos are open, doing and done

Todos are stored in $TODO_FILE (default: ~/.todo.txt).";

// Turning the result into an exit code holds no rule for Flux to check.
// Ignored code may call checked code, though not the other way round.
#[flux_rs::ignore]
fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    match run(&args, &mut io::stdout().lock()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("todo: {message}");
            ExitCode::FAILURE
        }
    }
}

defs! {
    use crate::list::need_with;
}

// What the arguments of `rm` are read into, said here and not on the enum
// and the function: detached specs, the same promises from outside the code.
#[specs {
    #[refined_by(need: int)]
    enum Numbers {
        One(usize[@index]) -> Numbers[index + 1],
        Run(usize[@first], {usize[@last] | first <= last}) -> Numbers[last + 1],
    }
    fn indexes_of(list: &TodoList[@n], arg: &str) -> Result<Vec<usize{i: i < n}>, String>;
}]
const _: () = ();

/// What the user asked for, with every todo number already turned into an
/// index. The refinement `need` is the shortest list the command can run on,
/// so a command that `parse` checked against a list runs on it with no
/// further checks. The invariant holds for every variant: no command needs
/// a list of negative length, so every command runs on some list.
#[refined_by(need: int)]
#[invariant(need >= 0)]
enum Command {
    #[variant((_) -> Command[0])]
    List(StatusSet),
    #[variant(Command[0])]
    Help,
    #[variant((_) -> Command[0])]
    Show(&'static dyn Choice),
    #[variant((_) -> Command[0])]
    Add(Title),
    #[variant((usize[@index], Status) -> Command[index + 1])]
    SetStatus(usize, Status),
    #[variant((usize[@index], Priority) -> Command[index + 1])]
    SetPriority(usize, Priority),
    #[variant((usize[@from], usize[@to]) -> Command[need_with(from + 1, to)])]
    Move(usize, usize),
    #[variant((Picks[@picks]) -> Command[picks.need])]
    Remove(Picks),
    #[variant(Command[0])]
    Clear,
    #[variant(Command[0])]
    Count,
}

/// Proved from the invariant alone, for a command built any way at all.
/// Only Flux compiles this.
#[cfg(flux)]
#[allow(dead_code)]
#[spec(fn(&Command[@c]) -> bool[c.need >= 0])]
fn no_command_needs_a_negative_length(_command: &Command) -> bool {
    true
}

/// Runs one command, writing what the user should see to `out`. Taking the
/// output as a trait object keeps this function free of the terminal.
fn run(args: &[String], out: &mut dyn Write) -> Result<(), String> {
    let path = todo_file()?;
    let mut list = load(&path)?;

    match parse(&list, args)? {
        Command::List(only) => {
            return written(print_list(out, &list, only));
        }
        Command::Help => {
            return written(writeln!(out, "{USAGE}"));
        }
        Command::Count => {
            let counts = list.by_status();
            return written(writeln!(
                out,
                "{} open, {} doing, {} done",
                counts[Status::Open.slot()],
                counts[Status::Doing.slot()],
                counts[Status::Done.slot()]
            ));
        }
        Command::Show(choice) => {
            return written(match choice.of(&list) {
                Some(index) => writeln!(
                    out,
                    "{:>width$}. {}",
                    list.position(index),
                    list[index].line(),
                    width = number_width(list.len())
                ),
                None => writeln!(out, "{}", choice.none()),
            });
        }
        Command::Add(title) => list.add(title),
        Command::SetStatus(index, to) => {
            if !list.set_status(index, to) {
                let todo = &list[index];
                return Err(format!(
                    "'{}' is {} and cannot become {}; undo it first",
                    todo.title.as_str(),
                    todo.status.name(),
                    to.name()
                ));
            }
        }
        Command::SetPriority(index, priority) => list.set_priority(index, priority),
        Command::Move(from, to) => list.move_to(from, to),
        Command::Remove(picks) => written(print_removed(out, list.remove_all(&picks)))?,
        Command::Clear => {
            let done = StatusSet::none().with(Status::Done);
            written(print_removed(out, list.remove_all(&done)))?;
        }
    }

    fs::write(&path, list.to_text())
        .map_err(|e| format!("cannot write {}: {e}", path.display()))?;
    written(print_list(out, &list, StatusSet::all()))
}

/// Reads the arguments as a command that `list` is long enough for.
#[spec(fn(&TodoList[@n], _) -> Result<Command{c: c.need <= n}, _>)]
fn parse(list: &TodoList, args: &[String]) -> Result<Command, String> {
    let (name, rest) = match args.split_first() {
        Some((name, rest)) => (name.as_str(), rest),
        None => ("list", args),
    };
    match (name, rest) {
        ("list", []) => Ok(Command::List(StatusSet::all())),
        ("list", names) => {
            let mut only = StatusSet::none();
            for name in names {
                match Status::named(name) {
                    Some(status) => only = only.with(status),
                    None => return Err(format!("'{name}' is not open, doing or done")),
                }
            }
            Ok(Command::List(only))
        }
        ("clear", []) => Ok(Command::Clear),
        ("count", []) => Ok(Command::Count),
        ("help" | "-h" | "--help", _) => Ok(Command::Help),
        ("next", []) => Ok(Command::Show(&MostUrgent)),
        ("now", []) => Ok(Command::Show(&InProgress)),
        ("add", words) => match Title::new(&words.join(" ")) {
            Some(title) => Ok(Command::Add(title)),
            None => Err("add needs a title".to_string()),
        },
        ("start", [n]) => Ok(Command::SetStatus(index_of(list, n)?, Status::Doing)),
        ("done", [n]) => Ok(Command::SetStatus(index_of(list, n)?, Status::Done)),
        ("undo", [n]) => Ok(Command::SetStatus(index_of(list, n)?, Status::Open)),
        ("pri", [n, level]) => Ok(Command::SetPriority(
            index_of(list, n)?,
            priority_of(level)?,
        )),
        ("mv", [n, m]) => Ok(Command::Move(index_of(list, n)?, index_of(list, m)?)),
        ("rm", numbers) if !numbers.is_empty() => {
            let mut picks = Picks::none();
            for numbers in numbers {
                for index in indexes_of(list, numbers)? {
                    if !picks.pick(index) {
                        return Err(format!("todo {} is named twice", list.position(index)));
                    }
                }
            }
            Ok(Command::Remove(picks))
        }
        _ => Err(format!("cannot run '{}'\n{USAGE}", args.join(" "))),
    }
}

fn todo_file() -> Result<PathBuf, String> {
    if let Some(path) = env::var_os("TODO_FILE") {
        return Ok(PathBuf::from(path));
    }
    match env::var_os("HOME") {
        Some(home) => Ok(PathBuf::from(home).join(".todo.txt")),
        None => Err("set TODO_FILE or HOME to say where todos are stored".to_string()),
    }
}

/// A missing file is an empty list, so the first `todo add` just works.
fn load(path: &PathBuf) -> Result<TodoList, String> {
    match fs::read_to_string(path) {
        Ok(text) => Ok(TodoList::parse(&text)),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(TodoList::new()),
        Err(e) => Err(format!("cannot read {}: {e}", path.display())),
    }
}

/// Turns the 1-based todo number a user typed into a proved-valid index.
#[spec(fn(&TodoList[@n], _) -> Result<usize{i: i < n}, _>)]
fn index_of(list: &TodoList, arg: &str) -> Result<usize, String> {
    let Ok(pos) = arg.parse() else {
        return Err(format!("'{arg}' is not a todo number"));
    };
    match list.locate(pos) {
        Some(index) => Ok(index),
        None => Err(format!("no todo {pos} (the list has {})", list.len())),
    }
}

/// The indexes an argument of `rm` names: one todo number, or a range of
/// them written `2-4`.
fn indexes_of(list: &TodoList, arg: &str) -> Result<Vec<usize>, String> {
    let numbers = match arg.split_once('-') {
        Some((first, last)) if !first.is_empty() => {
            let first = index_of(list, first)?;
            let last = index_of(list, last)?;
            if first > last {
                return Err(format!("the range {arg} runs backwards"));
            }
            Numbers::Run(first, last)
        }
        _ => Numbers::One(index_of(list, arg)?),
    };
    Ok(numbers.indexes())
}

/// What one argument of `rm` names: one todo, or a run of them from a first
/// to a last. A run that ends before it starts cannot be built, and `need`
/// is the shortest list all the todos named are in.
enum Numbers {
    One(usize),
    Run(usize, usize),
}

impl Numbers {
    #[spec(fn(Numbers[@need]) -> Vec<usize{i: i < need}>)]
    fn indexes(self) -> Vec<usize> {
        match self {
            Numbers::One(index) => span(index, index),
            Numbers::Run(first, last) => span(first, last),
        }
    }
}

/// The indexes from `first` to `last`, both included: every one between
/// them and exactly as many as that, so a range is never one short or one
/// over.
// The loop's invariant, how many indexes are collected so far, has the same
// shape as the length the spec promises. Scraping lets Flux try the shapes
// it sees in the specs, and so find it.
#[opts(scrape_quals = "true")]
#[spec(fn(first: usize, last: usize{first <= last}) -> Vec<usize{i: first <= i && i <= last}>[last - first + 1])]
fn span(first: usize, last: usize) -> Vec<usize> {
    let mut indexes = Vec::new();
    let mut index = first;
    while index < last {
        indexes.push(index);
        index += 1;
    }
    indexes.push(last);
    indexes
}

#[spec(fn(_) -> Result<Priority, _>)]
fn priority_of(arg: &str) -> Result<Priority, String> {
    if let Ok(n) = arg.parse()
        && let Some(priority) = priority(n)
    {
        return Ok(priority);
    }
    Err(format!(
        "priority must be {HIGHEST_PRIORITY} to {LOWEST_PRIORITY}, not '{arg}'"
    ))
}

/// A reader that stops early, as in `todo list | head -1`, is not an error.
fn written(result: io::Result<()>) -> Result<(), String> {
    match result {
        Err(e) if e.kind() != io::ErrorKind::BrokenPipe => Err(format!("cannot write output: {e}")),
        _ => Ok(()),
    }
}

/// How many characters the numbers of a list of `total` todos are printed
/// in, so that they line up: as many as the largest takes, and at least two.
#[spec(fn(total: usize) -> usize{width: 2 <= width && width <= total + 2})]
fn number_width(total: usize) -> usize {
    let mut width = 1;
    let mut rest = total;
    while rest >= 10 {
        // What stays true on every turn, which Flux does not find alone:
        // each digit counted was taken off what is left of the number.
        invariant!(width: int, rest: int; width <= total - rest + 1);
        rest /= 10;
        width += 1;
    }
    max(width, 2)
}

fn print_removed(out: &mut dyn Write, removed: Removed) -> io::Result<()> {
    for todo in removed {
        writeln!(out, "removed: {}", todo.title.as_str())?;
    }
    Ok(())
}

/// Prints the todos in the statuses of `only`, under the numbers the other commands take, then the
/// summary of the whole list.
fn print_list(out: &mut dyn Write, list: &TodoList, only: StatusSet) -> io::Result<()> {
    let total = list.len();
    if total == 0 {
        return writeln!(out, "nothing to do");
    }
    let width = number_width(total);
    let mut result = Ok(());
    list.each_index(|index| {
        let todo = &list[index];
        if result.is_ok() && only.contains(todo.status) {
            result = writeln!(out, "{:>width$}. {}", list.position(index), todo.line());
        }
    });
    result?;
    let progress = list.progress();
    writeln!(
        out,
        "{}/{total} done ({}%), {} left, {} doing",
        progress.done,
        progress.percent().part,
        progress.left(),
        list.by_status()[Status::Doing.slot()]
    )?;
    let open = list.open_by_priority();
    for priority in priorities() {
        writeln!(out, "  {} open at !{priority}", open[slot(priority)])?;
    }
    Ok(())
}
