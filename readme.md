Simple and fast cli daily task manager written in rust.  

`// Assuming you have cargo installed.` 
> In root directory, run `cargo install --path .`

```
Usage:  rustask [COMMAND] [ARGUMENTS]
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
```

