//! A text file reader which allows the user using `Up` and `Down` to scroll the screen.
#![no_std]

extern crate alloc;
extern crate task;
extern crate getopts;
extern crate path;
extern crate fs_node;
extern crate keycodes_ascii;
extern crate libterm;
extern crate spin;
extern crate app_io;
extern crate stdio;
extern crate core2;
extern crate shell;
//extern crate terminal_size;
//extern crate text_display;

//#[macro_use] extern crate log;

use fs_node::{FileOrDir};
use path::Path;
//use app_io::println;

// use keycodes_ascii::{Keycode, KeyAction};
use core::str;
use alloc::{
    string::String,
    vec::Vec,
};
use alloc::vec;
use alloc::format;
use alloc::string::ToString;
use app_io::println;
use getopts::Options;
//use terminal_size::{terminal_size, Width, Height};
// use getopts::Options;
// use path::Path;
// use fs_node::FileOrDir;
use alloc::collections::BTreeMap;
use libterm::Terminal;
//use window::Window;
//use text_display::TextDisplay;
//use libterm::cursor::Cursor;
use alloc::sync::Arc;
use spin::Mutex;
use stdio::KeyEventQueueReader;
use keycodes_ascii::Keycode;
use keycodes_ascii::KeyAction;
use shell::Shell;
// use stdio::{StdioWriter, KeyEventQueueReader};
// use core2::io::Write;


// The metadata for each line in the file.
struct LineSlice {
  // The starting index in the String for a line. (inclusive)
  start: usize,
  // The ending index in the String for a line. (exclusive)
  end: usize
}

//fn get_terminal_dimensions() -> Option<(usize, usize)> {
//    match terminal_size() {
//        Some((Width(width), Height(height))) => Some((width as usize, height as usize)),
//        None => None,  // If terminal size could not be determined
//    }
//}

// /// Read the whole file to a String.
fn get_content_string(file_path: String) -> Result<String, String> {
    let Ok(curr_wd) = task::with_current_task(|t| t.get_env().lock().working_dir.clone()) else {
        return Err("failed to get current task".to_string());
    };
    let path = Path::new(file_path.as_str());
    
    // navigate to the filepath specified by first argument
    match path.get(&curr_wd) {

        Some(file_dir_enum) => {
            match file_dir_enum {
                FileOrDir::Dir(directory) => {
                    Err(format!("{:?} a directory, cannot 'less' non-files.", directory.lock().get_name()))
                }
                FileOrDir::File(file) => {
                    let mut file_locked = file.lock();
                    let file_size = file_locked.len();
                    let mut string_slice_as_bytes = vec![0; file_size];
                    let _num_bytes_read = match file_locked.read_at(&mut string_slice_as_bytes, 0) {
                        Ok(num) => num,
                        Err(e) => {
                            println!("Failed to read error ");
                            return Err(format!("Failed to file size: {:?}", e));
                        }
                    };
                    let read_string = match str::from_utf8(&string_slice_as_bytes) {
                        Ok(string_slice) => string_slice,
                        Err(utf8_err) => {
                            println!("File was not a printable UTF-8 text file");
                            return Err(format!("Failed to read file: {:?}", utf8_err));
                        }
                    };
                    //println!("{}", read_string);
                    Ok(read_string.to_string())
                }
            }
        },
        None => {
             // Handle the case where the path wasn't found
             //         // For example, you could return an error or print a message:
             Err(format!("Couldn't find file at path".to_string()))                     
        }
        //_ => {
            //println!("Couldn't find file at path {}", path)
        //}
    }
}

// /// This function parses the text file. It scans through the whole file and records the string slice
// /// for each line. This function has full UTF-8 support, which means that the case where a single character
// /// occupies multiple bytes are well considered. The slice index returned by this function is guaranteed
// /// not to cause panic.
fn parse_content(content: &String) -> Result<BTreeMap<usize, LineSlice>, &'static str> {
     // Get the width and height of the terminal screen.
     //let (width, _height) = get_terminal_dimensions()
     //            .ok_or("couldn't get terminal dimensions")?;e

     // let (width, _height) = terminal.lock().get_text_dimensions();
     //let mut t = app_io::get_my_terminal().
     //                    get(&task::get_my_current_task_id())
     //                    .map(|property| property.terminal.clone());

     let mut terminal = Terminal::new().expect("Failed to create terminal");
     let (width, height) = terminal.get_text_dimensions();
     println!("{} {}", width, height);

     // println!("{} {}", width, _height);
     // Record the slice index of each line.
     let mut map: BTreeMap<usize, LineSlice> = BTreeMap::new();
     // Number of the current line.
     let mut cur_line_num: usize = 0;
     // Number of characters in the current line.
     let mut char_num_in_line: usize = 0;
     // Starting index in the String of the current line.
     let mut line_start_idx: usize = 0;
     // The previous character during the iteration. Set '\0' as the initial value since we don't expect
     // to encounter this character in the beginning of the file.
     let mut previous_char: char = '\0';

     // Iterate through the whole file.
     // `c` is the current character. `str_idx` is the index of the first byte of the current character.
     for (str_idx, c) in content.char_indices() {
         // When we need to begin a new line, record the previous line in the map.
         if char_num_in_line == width || previous_char == '\n' {
             map.insert(cur_line_num, LineSlice{ start: line_start_idx, end: str_idx });
             char_num_in_line = 0;
             line_start_idx = str_idx;
             cur_line_num += 1;
         }
         char_num_in_line += 1;
         previous_char = c;
     }
     map.insert(cur_line_num, LineSlice{ start: line_start_idx, end: content.len() });

     Ok(map)
}

// /// Display part of the file (may be whole file if the file is short) to the terminal, starting
// /// at line number `line_start`.
fn display_content(content: &str, map: &BTreeMap<usize, LineSlice>,
                    line_start: usize, terminal: &Terminal)
                    -> Result<(), &'static str> {
     // Get exclusive control of the terminal. It is locked through the whole function to
     // avoid the overhead of locking it multiple times.
     let mut locked_terminal = Terminal::new().expect("Failed to create terminal");
     // let mut locked_terminal = terminal.lock();

     // Calculate the last line to display. Make sure we don't extend over the end of the file.
     let (_width, height) = locked_terminal.get_text_dimensions();
     let mut line_end: usize = line_start + height;
     if line_end > map.len() {
         line_end = map.len();
     }

     // Refresh the terminal with the lines we've selected.
     let start_indices = match map.get(&line_start) {
         Some(indices) => indices,
         None => return Err("failed to get the byte indices of the first line")
     };
     let end_indices = match map.get(&(line_end - 1)) {
         Some(indices) => indices,
         None => return Err("failed to get the byte indices of the last line")
     };
     locked_terminal.clear();
     locked_terminal.print_to_terminal(
         content[start_indices.start..end_indices.end].to_string()
     );
     locked_terminal.refresh_display()
}

// /// Handle user keyboard strikes and perform corresponding operations.
fn event_handler_loop(content: &String, map: &BTreeMap<usize, LineSlice>,
                       key_event_queue: &KeyEventQueueReader)
                       -> Result<(), &'static str> {
     // Get a reference to this task's terminal. The terminal is *not* locked here.
     //let terminal = app_io::get_my_terminal().ok_or("couldn't get terminal for `less` app")?;
     let mut terminal = Terminal::new().expect("Failed to create terminal");

     // Display the beginning of the file.
     let mut line_start: usize = 0;
     display_content(content, map, 0, &terminal)?;

     // Handle user keyboard strikes.
     loop {
         match key_event_queue.read_one() {
             Some(keyevent) => {
                 if keyevent.action != KeyAction::Pressed { continue; }
                 match keyevent.keycode {
                     // Quit the program on "Q".
                     Keycode::Q => {
                         //let mut locked_terminal = terminal.lock();
                         //locked_terminal.clear();
                         return terminal.refresh_display()
                     },
                     // Scroll down a line on "Down".
                     Keycode::Down => {
                         if line_start + 1 < map.len() {
                             line_start += 1;
                         }
                         display_content(content, map, line_start, &terminal)?;
                     },
                     // Scroll up a line on "Up".
                     Keycode::Up => {
                         if line_start > 0 {
                             line_start -= 1;
                         }
                         display_content(content, map, line_start, &terminal)?;
                     }
                     _ => {}
                 }
             },
             _ => {}
         }
     }
}


pub fn main(args: Vec<String>) -> isize {

    // // Get stdout.
    let stdout = match app_io::stdout() {
         Ok(stdout) => stdout,
         Err(e) => {
             println!("{}", e);
             return 1;
         }
    };

    // // Set and parse options.
    let mut opts = Options::new();
    opts.optflag("h", "help", "print this help menu");
    let matches = match opts.parse(args) {
        Ok(m) => m,
        Err(e) => {
            //Err(format!("Is a directory, cannot 'less' non-files."))
            format!("{}", e);
            //print_usage(opts, stdout);
            return -1;
        }
    };
    if matches.opt_present("h") {
        //print_usage(opts, stdout);
        return 0;
    }
    if matches.free.is_empty() {
        //print_usage(opts, stdout);
        return 0;
    }
    let filename = matches.free[0].clone();
    
    let content = get_content_string(filename);
    
    match content {
        Ok(content) => {
            let map = parse_content(&content); // Now `content` is a `String`, and `&content` is a `&String`
            let shell = Shell::new_editor(content).expect("Failed to create new editor shell");
            shell.start().unwrap();
        },
        Err(e) => {
            // Handle the error (e.g.,)
            println!("Error: {}", e);
        }
    }

    //let terminal = Arc::new(Mutex::new(Terminal::new().expect("Failed to create terminal")));
    //let mut locked_terminal = terminal.lock().expect("failed to lock terminal");

    //let mut locked_terminal = Terminal::new().expect("Failed to create termina;");
    //let message = "Hello, Theseus!";
    //locked_terminal.print_to_terminal(message.to_string());
    //locked_terminal.refresh_display().unwrap();

    //if let Err(e) = run(filename) {
    //    error!("{}", e);
    //    return 1;
    //}
    0
}

//fn run(filename: String) -> Result<(), String> {

     // Acquire key event queue.
//     let key_event_queue = app_io::take_key_event_queue()?;
//     let key_event_queue = (*key_event_queue).as_ref()
//                           .ok_or("failed to take key event reader")?;

     // Read the whole file to a String.
//     let content = get_content_string(filename)?;

//     // Get it run.
//     let map = parse_content(&content)?;
//     Ok(event_handler_loop(&content, &map, key_event_queue)?)
//}

//fn print_usage(opts: Options, stdout: StdioWriter) {
//    let _ = stdout.lock().write_all(format!("{}\n", opts.usage(USAGE)).as_bytes());
//}

//const USAGE: &'static str = "Usage: less file
//read files";
