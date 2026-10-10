//! `todo` — a command-line todo list whose rules are verified by Flux.

// Linking these crates is what brings the std refinement specs into scope.
extern crate flux_alloc;
extern crate flux_core;

mod list;
#[cfg(flux)]
mod rejected;

use std::{env, fs, io, path::PathBuf, process::ExitCode};

use flux_rs::attrs::*;
use list::{
    HIGHEST_PRIORITY, LOWEST_PRIORITY, Priority, Status, Title, TodoList, priorities, priority,
    slot,
};

const USAGE: &str = "\
usage: todo [list]          show all todos
       todo list <status>   show only open, doing or done todos
       todo add <title>     add a todo
       todo start <n>       mark todo n as being worked on
       todo done <n>        mark todo n as done
       todo undo <n>        mark todo n as open again
       todo pri <n> <1-3>   set the priority of todo n (1 is highest)
       todo mv <n> <m>      move todo n to position m
       todo rm <n>          delete todo n

Todos are stored in $TODO_FILE (default: ~/.todo.txt).";

// Turning the result into an exit code holds no rule for Flux to check.
// Ignored code may call checked code, though not the other way round.
#[flux_rs::ignore]
fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    match run(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("todo: {message}");
            ExitCode::FAILURE
        }
    }
}

/// What the user asked for, with every todo number already turned into an
/// index. The refinement `need` is the shortest list the command can run on,
/// so a command that `parse` checked against a list runs on it with no
/// further checks.
#[refined_by(need: int)]
enum Command {
    #[variant((Option<Status>) -> Command[0])]
    List(Option<Status>),
    #[variant(Command[0])]
    Help,
    #[variant((Title) -> Command[0])]
    Add(Title),
    #[variant((usize[@index], Status) -> Command[index + 1])]
    SetStatus(usize, Status),
    #[variant((usize[@index], Priority) -> Command[index + 1])]
    SetPriority(usize, Priority),
    #[variant((usize[@from], usize[@to]) -> Command[if from > to { from + 1 } else { to + 1 }])]
    Move(usize, usize),
    #[variant((usize[@index]) -> Command[index + 1])]
    Remove(usize),
}

fn run(args: &[String]) -> Result<(), String> {
    let path = todo_file()?;
    let mut list = load(&path)?;

    match parse(&list, args)? {
        Command::List(only) => {
            print_list(&list, only);
            return Ok(());
        }
        Command::Help => {
            println!("{USAGE}");
            return Ok(());
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
        Command::Remove(index) => println!("removed: {}", list.remove(index).title.as_str()),
    }

    fs::write(&path, list.to_text())
        .map_err(|e| format!("cannot write {}: {e}", path.display()))?;
    print_list(&list, None);
    Ok(())
}

/// Reads the arguments as a command that `list` is long enough for.
#[spec(fn(&TodoList[@n], _) -> Result<Command{c: c.need <= n}, _>)]
fn parse(list: &TodoList, args: &[String]) -> Result<Command, String> {
    let (name, rest) = match args.split_first() {
        Some((name, rest)) => (name.as_str(), rest),
        None => ("list", args),
    };
    match (name, rest) {
        ("list", []) => Ok(Command::List(None)),
        ("list", [status]) => match Status::named(status) {
            Some(status) => Ok(Command::List(Some(status))),
            None => Err(format!("'{status}' is not open, doing or done")),
        },
        ("help" | "-h" | "--help", _) => Ok(Command::Help),
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
        ("rm", [n]) => Ok(Command::Remove(index_of(list, n)?)),
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

/// Prints the todos, all of them or only those in one status, under the
/// numbers the other commands take, then the summary of the whole list.
fn print_list(list: &TodoList, only: Option<Status>) {
    let total = list.len();
    if total == 0 {
        println!("nothing to do");
        return;
    }
    list.each_index(|index| {
        let todo = &list[index];
        if only.is_none_or(|status| status == todo.status) {
            println!("{:>2}. {}", index + 1, todo.line());
        }
    });
    let progress = list.progress();
    println!(
        "{}/{total} done ({}%), {} left",
        progress.done,
        progress.percent(),
        progress.left()
    );
    let open = list.open_by_priority();
    for priority in priorities() {
        println!("  {} open at !{priority}", open[slot(priority)]);
    }
}
