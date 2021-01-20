use std::env::args_os;
use std::fs;

fn main() {
    let path = args_os().nth(1).unwrap();
    let file = fs::read_to_string(path).unwrap();

    println!("{:#?}", hsmusicifier::parse::parse_album(&file));
}
