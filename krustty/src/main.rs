
use rtrb::CopyToUninit;

use crate::terminal::Terminal;

const MAX_LINE_LENGTH: usize = 4096;


mod terminal;
fn main() {
    let mut term = Terminal::spawn("zsh");

    let mut buffer = String::with_capacity(MAX_LINE_LENGTH);
    let stdin = std::io::stdin();


    println!("You can now type commands for Bash (type 'exit' to quit):");

    // Main thread sends user input to the writer thread.
    loop {
        buffer.clear();
        let num = stdin.read_line(&mut buffer).unwrap();
        if buffer.trim() == "exit" {
            break;
        }
        if let Ok(mut chunk) = term.input.write_chunk_uninit(num){
            let (slice1, slice2) = chunk.as_mut_slices();
            let wrap = slice1.len();
            buffer.as_bytes()[..wrap].copy_to_uninit(slice1);
            buffer.as_bytes()[wrap..].copy_to_uninit(slice2);
            unsafe {chunk.commit(num)};
        };
    }

    term.close();
}
