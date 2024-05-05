use std::env::args;
use std::process::exit;
use std::{fs, vec};

mod l5x;
mod udt;

// TODO: Improve organization
fn main() {
    let mut env_args = args().skip(1); // Skip the first argument, which will always be the program name
    let mut input_path: Option<String> = None;
    let mut output_path: Option<String> = None;

    let help = "This is a tool for converting UDT files exported from TIA Portal\n\
                        to an L5X XML format to import into Studio 5000\n\
                        The following are valid options:\n\
                        -i | --input  : Specify a UDT file to use as input\n\
                        -o | --output : Specify the location and name to save the L5X\n\
                        -h | --help   : Show this help dialogue";

    // Show help dialogue if no arguments are given
    if args().count() < 2 {
        println!("{}", help);
        exit(0);
    }

    // Handle CLI arguments
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

    // Try to open specified input file
    let input = if let Some(path) = input_path {
        fs::read_to_string(path).expect("Invalid input path!")
    } else {
        panic!("No input file specified!");
    };

    // Generate regex patterns before loop to avoid repeatedly compiling them
    let udt_regex = udt::build_udt_regex();
    let member_regex = udt::build_member_regex();

    // Generate empty Vec to be filled with parsed UDTs
    let mut udts: Vec<udt::Udt> = vec![];

    // Separate input into individual UDTs
    for udt_str in udt_regex.captures_iter(&input) {
        // Add UDT to udts Vec
        udts.push(udt::Udt {
            name: udt_str["udt_type"].into(),
            description: udt::get_udt_description(&udt_str),
            _version: udt_str["udt_version"].into(),
            members: vec![],
        });

        //Parse members in UDT body
        let mut target_nums = udt::BoolTargets::new();
        let body: String = udt_str["udt_body"].into();

        for member_str in member_regex.captures_iter(&body) {
            let data_type: String = udt::convert_type(&member_str["member_type"]).into();
            let bounds = udt::get_bounds(&member_str);
            let target = udt::get_target(&member_str, &mut udts, &target_nums);

            udts.last_mut()
                .expect("No UDTs found!")
                .members
                .push(udt::UdtMember {
                    name: member_str["member_name"].into(),
                    description: udt::get_member_description(&member_str),
                    data_type: data_type.clone(),
                    array_bounds: bounds.clone(),
                    external_write: udt::external_write(&member_str),
                    external_read: udt::external_read(&member_str),
                    hidden: false,
                    target: target.clone(),
                    bit_num: if data_type.to_uppercase() == "BOOL" && bounds == None {
                        Some(target_nums.bit_num)
                    } else {
                        None
                    },
                });

            if let &Some(_) = &target {
                target_nums.inc();
            }
        }
    }

    let parent_udt = udts.pop().unwrap();

    // Not elegant, but it properly adds the xml declaration to the beginning of the file
    let mut xml: Vec<u8> = "<?xml version=\"1.0\" ?>\n".into();
    xml.append(
        &mut l5x::create_l5x(&udts, parent_udt)
            .unwrap()
            .into_inner()
            .into_inner(),
    );

    // let result = writer.into_inner().into_inner();
    fs::write(output_path.unwrap(), xml).unwrap();
}
