use std::env;
use std::fs;
use std::io;
use std::path::Path;

use chrono::Local;
use rustask::{help, Todo};

fn main() -> io::Result<()> {
    let args: Vec<String> = env::args().collect();
    let date = Local::now().date_naive().to_string();

    ensure_directories()?;

    let main_file = format!("files/tasks/{}.txt", date);
    let backup_file = format!("files/backups/{}.txt", date);

    let todo = Todo::new(main_file, backup_file);
    todo.init_files()?;

    if args.len() > 1 {
        let command = args[1].as_str();
        match command {
            "show" | "list"     => todo.show()?,
            "add"               => todo.add(&args[2..])?,
            "rm"                => todo.remove(&args[2..])?,
            "edit"              => todo.edit(&args[2..])?,
            "rmrf" | "reset"    => todo.remove_all()?,
            "fin" | "done"      => todo.mark_done(&args[2..])?,
            "not_fin" | "undo"  => todo.mark_undone(&args[2..])?,
            "sort"              => todo.sort()?,
            "restore"           => todo.restore_backup()?,
            "show_back" | "raw" => todo.show_backup()?,
            "journal"           => todo.journal()?,
            "help" | "--help" | "-h" => help()?,
            _ => {
                eprintln!("Unrecognized command: {}", command);
                help()?;
            }
        }
    } else {
        // Default if no args provided
        todo.show()?;
    }

    Ok(())
}

fn ensure_directories() -> io::Result<()> {
    if !Path::new("files/tasks/").exists() {
        fs::create_dir_all("files/tasks/")?;
    }
    if !Path::new("files/backups/").exists() {
        fs::create_dir_all("files/backups/")?;
    }
    Ok(())
}

