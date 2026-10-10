//! `todo` — a command-line todo list whose index safety is verified by Flux.

// Linking these crates is what brings the std refinement specs into scope.
extern crate flux_alloc;
extern crate flux_core;

mod list;

use std::{env, fs, io, path::PathBuf, process::ExitCode};

use list::TodoList;

const USAGE: &str = "\
usage: todo [list]        show all todos
       todo add <title>   add a todo
       todo done <n>      mark todo n as done
       todo undo <n>      mark todo n as not done
       todo rm <n>        delete todo n

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

    match command {
        "list" => {
            print_list(&list);
            return Ok(());
        }
        "add" => {
            let title = rest.join(" ");
            if title.trim().is_empty() {
                return Err("add needs a title".to_string());
            }
            list.add(title.trim());
        }
        "done" | "undo" | "rm" => {
            let pos = position(rest)?;
            let Some(index) = list.locate(pos) else {
                return Err(format!("no todo {pos} (the list has {})", list.len()));
            };
            match command {
                "rm" => println!("removed: {}", list.remove(index).title),
                _ => list.set_done(index, command == "done"),
            }
        }
        "help" | "-h" | "--help" => {
            println!("{USAGE}");
            return Ok(());
        }
        _ => return Err(format!("unknown command '{command}'\n{USAGE}")),
    }

    fs::write(&path, list.to_text())
        .map_err(|e| format!("cannot write {}: {e}", path.display()))?;
    print_list(&list);
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

fn position(args: &[String]) -> Result<usize, String> {
    match args {
        [n] => n.parse().map_err(|_| format!("'{n}' is not a todo number")),
        _ => Err("expected exactly one todo number".to_string()),
    }
}

fn print_list(list: &TodoList) {
    if list.len() == 0 {
        println!("nothing to do");
    }
    let mut i = 0;
    while i < list.len() {
        println!("{:>2}. {}", i + 1, list.get(i).line());
        i += 1;
    }
}
