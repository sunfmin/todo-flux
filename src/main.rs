//! `todo` — a command-line todo list whose index safety is verified by Flux.

// Linking these crates is what brings the std refinement specs into scope.
extern crate flux_alloc;
extern crate flux_core;

mod list;
#[cfg(flux)]
mod rejected;

use std::{env, fs, io, path::PathBuf, process::ExitCode};

use flux_rs::attrs::*;
use list::{HIGHEST_PRIORITY, LOWEST_PRIORITY, Priority, Status, Title, TodoList, priority, slot};

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

fn run(args: &[String]) -> Result<(), String> {
    let (command, rest) = match args.split_first() {
        Some((command, rest)) => (command.as_str(), rest),
        None => ("list", args),
    };
    let path = todo_file()?;
    let mut list = load(&path)?;

    match (command, rest) {
        ("list", []) => {
            print_list(&list, None);
            return Ok(());
        }
        ("list", [name]) => {
            let Some(status) = Status::named(name) else {
                return Err(format!("'{name}' is not open, doing or done"));
            };
            print_list(&list, Some(status));
            return Ok(());
        }
        ("help" | "-h" | "--help", _) => {
            println!("{USAGE}");
            return Ok(());
        }
        ("add", words) => {
            let Some(title) = Title::new(&words.join(" ")) else {
                return Err("add needs a title".to_string());
            };
            list.add(title);
        }
        ("start", [n]) => change_status(&mut list, n, Status::Doing)?,
        ("done", [n]) => change_status(&mut list, n, Status::Done)?,
        ("undo", [n]) => change_status(&mut list, n, Status::Open)?,
        ("pri", [n, level]) => {
            let index = index_of(&list, n)?;
            list.set_priority(index, priority_of(level)?);
        }
        ("mv", [n, m]) => {
            let from = index_of(&list, n)?;
            let to = index_of(&list, m)?;
            list.move_to(from, to);
        }
        ("rm", [n]) => {
            let index = index_of(&list, n)?;
            println!("removed: {}", list.remove(index).title.as_str());
        }
        _ => return Err(format!("cannot run '{}'\n{USAGE}", args.join(" "))),
    }

    fs::write(&path, list.to_text())
        .map_err(|e| format!("cannot write {}: {e}", path.display()))?;
    print_list(&list, None);
    Ok(())
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
#[spec(fn(&TodoList[@n], &str) -> Result<usize{i: i < n}, String>)]
fn index_of(list: &TodoList, arg: &str) -> Result<usize, String> {
    let Ok(pos) = arg.parse() else {
        return Err(format!("'{arg}' is not a todo number"));
    };
    match list.locate(pos) {
        Some(index) => Ok(index),
        None => Err(format!("no todo {pos} (the list has {})", list.len())),
    }
}

fn change_status(list: &mut TodoList, arg: &str, to: Status) -> Result<(), String> {
    let index = index_of(list, arg)?;
    if list.set_status(index, to) {
        return Ok(());
    }
    let from = list.get(index).status;
    Err(format!(
        "todo {arg} is {} and cannot become {}; undo it first",
        from.name(),
        to.name()
    ))
}

#[spec(fn(&str) -> Result<Priority, String>)]
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
        let todo = list.get(index);
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
    for n in HIGHEST_PRIORITY..=LOWEST_PRIORITY {
        if let Some(priority) = priority(n) {
            println!("  {} open at !{priority}", open[slot(priority)]);
        }
    }
}
