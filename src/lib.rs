use chrono::Local;
use std::fs::{self, File, OpenOptions};
use std::io::{self, BufRead, BufReader, BufWriter, Read, Write};
use std::path::Path;

pub struct Entry {
    pub task: String,
    pub start: String,
    pub end: String,
    pub done: bool,
}

impl Entry {
    pub fn new(task: String, start: String, end: String) -> Self {
        Self {
            task,
            start,
            end,
            done: false,
        }
    }
}

pub struct Todo {
    main_file: String,
    backup_file: String,
}

impl Todo {
    pub fn new(main_file: String, backup_file: String) -> Self {
        Self { main_file, backup_file }
    }

    pub fn init_files(&self) -> io::Result<()> {
        self.create_file_if_empty(&self.main_file)?;
        self.create_file_if_empty(&self.backup_file)?;
        Ok(())
    }

    pub fn show(&self) -> io::Result<()> {
        let contents = fs::read_to_string(&self.main_file).unwrap_or_default();
        if contents.trim().is_empty() {
            println!("\n( No tasks yet. )\n");
        } else {
            println!("\n{contents}");
        }
        Ok(())
    }

    pub fn show_backup(&self) -> io::Result<()> {
        let contents = fs::read_to_string(&self.backup_file).unwrap_or_default();
        if contents.trim().is_empty() {
            println!("\n( Backup is empty. )\n");
        } else {
            println!("\n{contents}");
        }
        Ok(())
    }

    pub fn add(&self, args: &[String]) -> io::Result<()> {
        // Expect exactly 3 args: <TASK> <START> <END>
        if args.len() < 3 {
            eprintln!("Usage: add <TASK> <START> <END>");
            return Ok(());
        }

        let entry = Entry::new(args[0].clone(), args[1].clone(), args[2].clone());
        self.add_to_file(&self.main_file, &entry)?;
        self.add_to_file(&self.backup_file, &entry)?;
        self.show()?;
        Ok(())
    }

    pub fn remove(&self, args: &[String]) -> io::Result<()> {
        let index = parse_index(args)?;
        if !self.remove_index_from_file(&self.main_file, index)? {
            eprintln!("Index {} not found in tasks.", index);
            return Ok(());
        }
        let _ = self.remove_index_from_file(&self.backup_file, index);
        self.show()?;
        Ok(())
    }

    pub fn edit(&self, args: &[String]) -> io::Result<()> {
        let index = parse_index(args)?;
        println!("Enter new <task_name> <start_time> <end_time>:");

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let fields: Vec<&str> = input.trim().split_whitespace().collect();

        if fields.len() != 3 {
            eprintln!("Invalid input. Need exactly: <task_name> <start_time> <end_time>.");
            return Ok(());
        }

        let new_task = fields[0];
        let new_start = fields[1];
        let new_end = fields[2];

        let mut buffer = BufReader::new(self.open_file(&self.main_file, true, false, false, false, false)?);
        let mut lines = Vec::new();
        buffer.read_to_end(&mut lines)?;
        let lines = String::from_utf8_lossy(&lines);
        let lines: Vec<String> = lines.lines().map(|l| l.to_string()).collect();

        let mut output = Vec::new();
        if let Some(header) = lines.get(0) {
            output.push(header.to_string());
        }

        let mut found = false;
        for line in lines.iter().skip(1) {
            if line.starts_with(&index.to_string()) {
                let edited_line = format!(
                    "{:<3} {:<30}   {:>20} | {}",
                    index,
                    new_task,
                    format!("{} - {}", new_start, new_end),
                    "✕"
                );
                output.push(edited_line);
                found = true;
            } else {
                output.push(line.to_string());
            }
        }

        let mut writer = BufWriter::new(self.open_file(&self.main_file, false, true, false, false, true)?);
        for line in output {
            writeln!(writer, "{}", line)?;
        }
        writer.flush()?;

        if found {
            self.update_backup()?;
            self.show()?;
        } else {
            eprintln!("Index {} not found in tasks.", index);
        }
        Ok(())
    }

    pub fn mark_done(&self, args: &[String]) -> io::Result<()> {
        let index = parse_index(args)?;
        self.set_completion_status(index, true)?;
        self.show()?;
        Ok(())
    }

    pub fn mark_undone(&self, args: &[String]) -> io::Result<()> {
        let index = parse_index(args)?;
        self.set_completion_status(index, false)?;
        self.show()?;
        Ok(())
    }

    pub fn sort(&self) -> io::Result<()> {
        let buffer = BufReader::new(self.open_file(&self.main_file, true, false, false, false, false)?);
        let lines: Vec<String> = buffer.lines().filter_map(Result::ok).collect();

        if lines.is_empty() {
            println!("No tasks to sort.");
            return Ok(());
        }

        let mut header = String::new();
        let mut incomplete = Vec::new();
        let mut complete = Vec::new();

        // Date header 
        if let Some(first) = lines.first() {
            header = first.clone();
        }

        for line in lines.iter().skip(1) {
            let is_done = line.ends_with('✓');
            if is_done {
                complete.push(line.to_string());
            } else {
                incomplete.push(line.to_string());
            }
        }

        let mut writer = BufWriter::new(self.open_file(&self.main_file, false, true, false, false, true)?);
        // Rewrite date header 
        writeln!(writer, "{}", header)?;
        let mut idx = 1;
        for line in &incomplete {
            let task_part = match line.get(4..) {
                Some(s) => s,
                None => line,
            };
            writeln!(writer, "{:<3}{}", idx, task_part)?;
            idx += 1;
        }
        // Rewrite complete tasks
        for line in &complete {
            let task_part = match line.get(4..) {
                Some(s) => s,
                None => line,
            };
            writeln!(writer, "{:<3}{}", idx, task_part)?;
            idx += 1;
        }
        writer.flush()?;

        self.update_backup()?;
        self.show()?;
        Ok(())
    }

    pub fn remove_all(&self) -> io::Result<()> {
        println!("Remove all tasks? Type `Yes` to confirm.");
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        if input.trim() == "Yes" {
            // Remove contents of main file 
            let _ = OpenOptions::new().write(true).truncate(true).open(&self.main_file)?;
            println!("All tasks removed.");
        } else {
            eprintln!("Cancelling.");
        }
        Ok(())
    }

    pub fn restore_backup(&self) -> io::Result<()> {
        println!("Merge backup into main file? Type `Yes` to confirm.");
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        if input.trim() == "Yes" {
            fs::copy(&self.backup_file, &self.main_file)?;
            self.show()?;
        } else {
            println!("Cancelling.");
        }
        Ok(())
    }

    pub fn journal(&self) -> io::Result<()> {
        println!("Write your thoughts for today:");
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;

        let mut writer = BufWriter::new(self.open_file(&self.main_file, false, true, true, false, false)?);
        writeln!(writer, "\nThoughts:")?;
        write!(writer, "{}", input)?;
        writer.flush()?;
        self.update_backup()?;
        println!("Journal entry saved.");
        Ok(())
    }

    // Helpers

    fn open_file(
        &self,
        file_path: &str,
        read: bool,
        write: bool,
        append: bool,
        create: bool,
        truncate: bool,
    ) -> io::Result<File> {
        OpenOptions::new()
            .read(read)
            .write(write)
            .append(append)
            .create(create)
            .truncate(truncate)
            .open(file_path)
    }

    fn update_backup(&self) -> io::Result<()> {
        fs::copy(&self.main_file, &self.backup_file)?;
        Ok(())
    }

    fn create_file_if_empty(&self, file_path: &str) -> io::Result<()> {
        let path = Path::new(file_path);
        if !path.exists() || fs::metadata(path)?.len() == 0 {
            let mut writer = BufWriter::new(self.open_file(file_path, false, true, false, true, false)?);
            let date = Local::now().date_naive().to_string();
            writeln!(writer, "{}", date)?;
            writer.flush()?;
        }
        Ok(())
    }

    fn add_to_file(&self, file_path: &str, entry: &Entry) -> io::Result<()> {
        let reader = BufReader::new(self.open_file(file_path, true, false, false, false, false)?);
        let line_count = reader.lines().count();

        let mut writer = BufWriter::new(self.open_file(file_path, false, true, true, false, false)?);
        let formatted = format!(
            "{:<3} {:<30}   {:>20} | {}",
            line_count,
            entry.task,
            format!("{} - {}", entry.start, entry.end),
            if entry.done { "✓" } else { "✕" },
        );
        writeln!(writer, "{}", formatted)?;
        Ok(())
    }

    fn remove_index_from_file(&self, file_path: &str, index: i8) -> io::Result<bool> {
        let buffer = BufReader::new(self.open_file(file_path, true, false, false, false, false)?);
        let lines: Vec<String> = buffer.lines().filter_map(Result::ok).collect();

        if lines.is_empty() {
            return Ok(false);
        }

        let mut output = Vec::new();
        // Keep Date
        if let Some(header) = lines.get(0) {
            output.push(header.clone());
        }

        let mut new_index = 1;
        let mut found = false;
        for line in lines.iter().skip(1) {
            if line.starts_with(&index.to_string()) {
                found = true;
                continue;
            }
            let text_after_index = match line.get(4..) {
                Some(s) => s,
                None => line,
            };
            let reconstructed = format!("{:<3}{}", new_index, text_after_index);
            output.push(reconstructed);
            new_index += 1;
        }

        let mut writer = BufWriter::new(self.open_file(file_path, false, true, false, false, true)?);
        for line in output {
            writeln!(writer, "{}", line)?;
        }
        writer.flush()?;

        Ok(found)
    }

    fn set_completion_status(&self, index: i8, done: bool) -> io::Result<()> {
        let buffer = BufReader::new(self.open_file(&self.main_file, true, false, false, false, false)?);
        let lines: Vec<String> = buffer.lines().filter_map(Result::ok).collect();

        if lines.is_empty() {
            eprintln!("No tasks found.");
            return Ok(());
        }

        let mut output = Vec::new();
        if let Some(header) = lines.get(0) {
            output.push(header.clone());
        }

        let mut found = false;
        for line in lines.iter().skip(1) {
            if !line.starts_with(&index.to_string()) {
                output.push(line.to_string());
            } else {
                let mut line_mod = line.clone();
                if !line_mod.is_empty() {
                    line_mod.pop(); 
                }
                let symbol = if done { "✓" } else { "✕" };
                line_mod.push_str(symbol);
                output.push(line_mod);
                found = true;
            }
        }

        if !found {
            eprintln!("Index {} not found in tasks.", index);
            return Ok(());
        }

        let mut writer = BufWriter::new(self.open_file(&self.main_file, false, true, false, false, true)?);
        for line in output {
            writeln!(writer, "{}", line)?;
        }
        writer.flush()?;
        self.update_backup()?;
        Ok(())
    }
}

// Helper: parse index from user arguments
fn parse_index(args: &[String]) -> io::Result<i8> {
    if args.is_empty() {
        eprintln!("No index provided. Usage example: rm 3");
        return Err(io::Error::new(io::ErrorKind::Other, "Missing index argument."));
    }

    match args[0].trim().parse::<i8>() {
        Ok(num) => Ok(num),
        Err(_) => {
            eprintln!("'{}' is not a valid number.", args[0]);
            Err(io::Error::new(io::ErrorKind::InvalidInput, "Invalid index format."))
        }
    }
}

const HELP: &str = "\
Usage: cargo run -- [COMMAND] [ARGUMENTS]

Commands:
  add <task> <start> <end>   Adds a new task
  edit <index>               Edits the given task by index
  rm <index>                 Removes a task by index
  rmrf / reset               Removes all tasks
  fin / done <index>         Marks the task as done
  not_fin / undo <index>     Marks the task as not done
  show / list                Displays tasks
  show_back / raw            Displays backup file
  restore                    Copies backup file over main file
  sort                       Sorts tasks by undone/done
  journal                    Appends a journal entry
  help                       Displays this message
";

pub fn help() -> io::Result<()> {
    println!("{}", HELP);
    Ok(())
}

