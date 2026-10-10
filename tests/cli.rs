use std::{
    env, fs,
    path::PathBuf,
    process::{Command, Output},
};

struct Cli {
    file: PathBuf,
}

impl Cli {
    fn new(name: &str) -> Cli {
        let file = env::temp_dir().join(format!("todo-test-{}-{name}.txt", std::process::id()));
        let _ = fs::remove_file(&file);
        Cli { file }
    }

    fn run(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_todo"))
            .args(args)
            .env("TODO_FILE", &self.file)
            .output()
            .unwrap()
    }

    fn ok(&self, args: &[&str]) -> String {
        let out = self.run(args);
        assert!(
            out.status.success(),
            "{args:?} failed: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        String::from_utf8(out.stdout).unwrap()
    }

    fn stored(&self) -> String {
        fs::read_to_string(&self.file).unwrap()
    }
}

impl Drop for Cli {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.file);
    }
}

#[test]
fn add_done_undo_rm_round_trip() {
    let cli = Cli::new("round-trip");
    assert_eq!(cli.ok(&[]), "nothing to do\n");

    cli.ok(&["add", "buy", "milk"]);
    cli.ok(&["add", "write report"]);
    assert_eq!(
        cli.ok(&["list"]),
        " 1. [ ] buy milk\n 2. [ ] write report\n"
    );

    assert_eq!(
        cli.ok(&["done", "2"]),
        " 1. [ ] buy milk\n 2. [x] write report\n"
    );
    assert_eq!(cli.stored(), "[ ] buy milk\n[x] write report\n");

    cli.ok(&["undo", "2"]);
    assert_eq!(cli.stored(), "[ ] buy milk\n[ ] write report\n");

    assert_eq!(
        cli.ok(&["rm", "1"]),
        "removed: buy milk\n 1. [ ] write report\n"
    );
    assert_eq!(cli.stored(), "[ ] write report\n");

    assert_eq!(
        cli.ok(&["rm", "1"]),
        "removed: write report\nnothing to do\n"
    );
    assert_eq!(cli.stored(), "");
}

#[test]
fn mv_reorders_without_losing_todos() {
    let cli = Cli::new("mv");
    for title in ["a", "b", "c"] {
        cli.ok(&["add", title]);
    }
    cli.ok(&["mv", "1", "3"]);
    assert_eq!(cli.stored(), "[ ] b\n[ ] c\n[ ] a\n");
    cli.ok(&["mv", "3", "1"]);
    assert_eq!(cli.stored(), "[ ] a\n[ ] b\n[ ] c\n");
    cli.ok(&["mv", "2", "2"]);
    assert_eq!(cli.stored(), "[ ] a\n[ ] b\n[ ] c\n");

    for args in [["mv", "0", "1"], ["mv", "1", "4"], ["mv", "4", "1"]] {
        assert!(!cli.run(&args).status.success(), "{args:?}");
    }
    assert_eq!(cli.stored(), "[ ] a\n[ ] b\n[ ] c\n");
}

#[test]
fn out_of_range_positions_are_rejected_without_changing_the_file() {
    let cli = Cli::new("out-of-range");
    cli.ok(&["add", "only one"]);

    for args in [
        ["done", "0"],
        ["done", "2"],
        ["rm", "2"],
        ["undo", "x"],
        ["rm", "-1"],
    ] {
        let out = cli.run(&args);
        assert!(!out.status.success(), "{args:?} should fail");
        assert!(out.stdout.is_empty());
    }
    assert_eq!(cli.stored(), "[ ] only one\n");
}

#[test]
fn bad_commands_fail() {
    let cli = Cli::new("bad-commands");
    assert!(!cli.run(&["add"]).status.success());
    assert!(!cli.run(&["add", "  "]).status.success());
    assert!(!cli.run(&["frobnicate"]).status.success());
    assert!(!cli.run(&["done"]).status.success());
    assert!(!cli.run(&["done", "1", "2"]).status.success());
    assert!(!cli.run(&["mv", "1"]).status.success());
    assert!(!cli.file.exists());
}

#[test]
fn hand_edited_file_loads() {
    let cli = Cli::new("hand-edited");
    fs::write(&cli.file, "no mark\n\n[x] finished\n").unwrap();
    cli.ok(&["mv", "1", "1"]);
    assert_eq!(cli.stored(), "[ ] no mark\n[x] finished\n");
}

#[test]
fn multi_line_title_stays_one_todo() {
    let cli = Cli::new("multi-line");
    cli.ok(&["add", "line one\nline two"]);
    assert_eq!(cli.stored(), "[ ] line one line two\n");
}
