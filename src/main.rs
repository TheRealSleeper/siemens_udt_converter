use std::env::args;
use std::fs;
use std::process::exit;

mod l5x;
mod udt;

fn main() {
    let mut env_args = args().skip(1);
    let mut input_path: Option<String> = None;
    let mut output_path: Option<String> = None;

    let help = "This is a tool for converting UDT files exported from TIA Portal\n\
                        to an L5X XML format to import into Studio 5000\n\
                        The following are valid options:\n\
                        -i | --input  : Specify a UDT file to use as input\n\
                        -o | --output : Specify the location and name to save the L5X\n\
                        -h | --help   : Show this help dialogue";

    if args().count() < 2 {
        println!("{}", help);
        exit(0);
    }

    while let Some(arg) = env_args.next() {
        match arg.as_str() {
            "-i" | "--input" => {
                input_path = Some(env_args.next().expect("No argument given for input path!"))
            }
            "-o" | "--output" => {
                output_path = Some(env_args.next().expect("No argument given for output path!"))
            }
            "-h" | "--help" => {
                println!("{}", help)
            }
            _ => {
                println!("Unknown command");
                exit(0);
            }
        }
    }

    if input_path == None {
        println!("No input path given!");
        exit(0);
    }

    if output_path == None {
        println!("No output path given!");
        exit(0);
    }

    let input = if let Some(path) = input_path {
        fs::read_to_string(path).expect("Invalid input path!")
    } else {
        panic!("No input file specified!");
    };

    let mut udts = udt::get_udts(input);

    let parent_udt = udts.pop().unwrap();

    // Not elegant, but it properly adds the xml declaration to the beginning of the file
    let mut xml: Vec<u8> = "<?xml version=\"1.0\" ?>\n".into();
    xml.append(
        &mut l5x::create_l5x(&udts, parent_udt)
            .unwrap()
            .into_inner()
            .into_inner(),
    );

    fs::write(output_path.unwrap(), xml).unwrap();
}
