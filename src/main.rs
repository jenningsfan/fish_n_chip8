use std::env;

mod cpu;
mod io;

fn main() {
    let args: Vec<String> = env::args().collect();
    io::emulator_main(args[1].clone());
}