use chrono::Local;
use std::fs::{self, File, OpenOptions};
use std::io::{self, BufRead, BufReader, BufWriter, Write};
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

    pub fn rm(&self, args: &[String]) -> io::Result<()> {
        let index = parse_index(args)?;
        if !self.remove_index_from_file(&self.main_file, index)? {
            eprintln!("{index} not found.");
            return Ok(());
        }

        let _ = self.remove_index_from_file(&self.backup_file, index)?;
        self.show()?;
        Ok(())
    }


    pub fn edit(&self, args: &[String]) -> io::Result<()> {
        let index = parse_index(args)?;
        println!("Enter new <task_name> <start_time> <end_time>:");

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let parts: Vec<&str> = input.trim().split_whitespace().collect();

        if parts.len() < 3 {
            eprintln!("Invalid input. Need at least 3 tokens: <task_name> <start> <end>.");
            return Ok(());
        }

        let end = parts.last().unwrap();
        let start = parts.get(parts.len() - 2).unwrap();
        let task = parts[..parts.len() - 2].join(" ");

        let lines: Vec<String> = BufReader::new(self.open_file(&self.main_file, true, false, false, false, false)?)
            .lines()
            .filter_map(Result::ok)
            .collect();

        let mut output = vec![lines[0].clone()];
        let mut found = false;

        for line in &lines[1..] {
            if line.starts_with(&index.to_string()) {
                let edited = format!(
                    "{:<3} {:<30}   {:>20} | ✕",
                    index,
                    task,
                format!("{} - {}", start, end)
                );
                output.push(edited);
                found = true;
            } else {
                output.push(line.clone());
            }
        }

        let mut w = BufWriter::new(self.open_file(&self.main_file, false, true, false, false, true)?);
        for l in output { writeln!(w, "{l}")?; }
        w.flush()?;

        if found {
            self.update_backup()?;
            self.show()?;
        } else {
            eprintln!("Index {} not found.", index);
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
        let file_path = &self.main_file; 
        let reader = BufReader::new(self.open_file(file_path, true, false, false, false, false)?);

        let lines: io::Result<Vec<String>> = reader.lines().collect(); 
        let mut lines = lines?; 

        let mut unfinished_lines: Vec<String> = Vec::new(); 
        let mut finished_lines  : Vec<String> = Vec::new(); 

        match lines.get(0) {
            Some(date) => unfinished_lines.push(date.to_string()), 
            None       => ()
        } 

        for line in lines.iter_mut().skip(1) { 
            if line.chars().last().unwrap() == '✕' { 
                unfinished_lines.push(line.to_string()); 
            } else { 
                finished_lines.push(line.to_string()); 
            }
        }

        let writer = self.open_file(file_path, false, true, false, false, true)?;
        let mut writer = BufWriter::new(writer);

        unfinished_lines.extend(finished_lines); 
        
        let mut lines_to_keep: Vec<String> = Vec::new(); 
        lines_to_keep.push(unfinished_lines.remove(0)); 

        let mut num_lines = 1; 
        for line in unfinished_lines.iter() {
            let task = line.get(4..).unwrap_or(""); 
            lines_to_keep.push(format!("{num_lines}    {task}"));
            num_lines += 1    
        }

        for line in lines_to_keep {
            writer.write_all(line.as_bytes())?;
            writer.write_all(b"\n")?;
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
        let reader = BufReader::new(self.open_file(file_path, true, false, false, false, false)?);
        let lines: Vec<String> = reader.lines().collect::<io::Result<_>>()?;
        let mut output = Vec::with_capacity(lines.len());

        //  Keep the date header 
        if let Some(header) = lines.get(0) {
            output.push(header.clone());
        }

        let idx_str = index.to_string();
        let mut new_index = 1;
        let mut found = false; 
        for line in lines.iter().skip(1) {
            if line.starts_with(&idx_str) {
                continue;
            }
            let task_text = line.get(4..).unwrap_or("").trim_end();
            output.push(format!("{:<3} {}", new_index, task_text));
            new_index += 1;
            found = true; 
        }

        let mut writer = BufWriter::new(self.open_file(file_path, false, true, false, false, true)?);
        for line in output {
            writeln!(writer, "{line}")?;
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

