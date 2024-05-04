use std::env::args;
use std::process::exit;
use std::{fs, vec};

mod l5x;
mod udt;

// TODO: Add better modularization
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
                exit(-1);
            }
        }
    }

    if input_path == None {
        println!("No input path given!");
        exit(-1);
    }

    if output_path == None {
        println!("No output path given!");
        exit(-1);
    }

    let udt_regex = udt::build_udt_regex();
    let member_regex = udt::build_member_regex();

    // Try to open specified input file
    let input = if let Some(path) = input_path {
        fs::read_to_string(path).expect("Invalid input path!")
    } else {
        panic!("No input file specified!");
    };

    // Generate empty Vec to be filled with parsed UDTs
    let mut udts: Vec<udt::Udt> = vec![];

    // Separate input into individual UDTs
    let udts_str = udt_regex.captures_iter(&input);
    for udt_str in udts_str {
        let name: String = udt_str["udt_type"].into();
        let description: Option<String> = if let Some(desc) = udt_str.name("udt_title") {
            Some(String::from(desc.as_str()))
        } else {
            None
        };
        let version: String = udt_str["udt_version"].into();
        let body: String = udt_str["udt_body"].into();

        // Write
        udts.push(udt::Udt {
            name: name.clone(),
            description: description.clone(),
            _version: version.clone(),
            members: vec![],
        });

        //Parse members in UDT body
        let mut target_num = 0;
        let mut bit_num = 0;
        let members_str = member_regex.captures_iter(&body);
        for member_str in members_str {
            let name: String = member_str["member_name"].into();
            let data_type: String = udt::convert_type(&member_str["member_type"]).into();

            let lower_bound: Option<String> = if let Some(bound) = member_str.name("bound_lower") {
                Some(String::from(bound.as_str()))
            } else {
                None
            };
            let upper_bound: Option<String> = if let Some(bound) = member_str.name("bound_upper") {
                Some(String::from(bound.as_str()))
            } else {
                None
            };
            let bounds: Option<(isize, isize)> =
                if let (Some(lower), Some(upper)) = (lower_bound, upper_bound) {
                    Some((
                        lower.parse().expect("Lower bound invalid format"),
                        upper.parse().expect("Upper bound invalid format"),
                    ))
                } else {
                    None
                };

            let description: Option<String> =
                if let Some(desc) = member_str.name("member_description") {
                    Some(String::from(desc.as_str()))
                } else {
                    None
                };

            let external_write = if let Some(ext_wrt) = member_str.name("ext_wrt") {
                if ext_wrt.as_str().to_lowercase() == "false" {
                    false
                } else {
                    true
                }
            } else {
                true
            };

            let external_read = if let Some(ext_vis) = member_str.name("ext_vis") {
                if ext_vis.as_str().to_lowercase() == "false" {
                    false
                } else {
                    true
                }
            } else {
                true
            };

            let mut target_name = "ZZZZZZZZZZ".to_string();
            target_name.push_str(&udts.last().unwrap().name);

            if let (true, None) = (data_type.to_uppercase() == "BOOL", bounds) {
                if bit_num == 8 {
                    bit_num = 0;
                    target_num += 1;
                }

                target_name.push_str(&target_num.to_string());

                if bit_num == 0 {
                    udts.last_mut().unwrap().members.insert(
                        target_num,
                        udt::UdtMember {
                            name: target_name.clone(),
                            description: None,
                            data_type: "SINT".to_string(),
                            array_bounds: None,
                            external_read: false,
                            external_write: false,
                            hidden: true,
                            target: None,
                            bit_num: None,
                        },
                    )
                }
            }

            let target = if data_type.to_uppercase() == "BOOL" {
                Some(target_name)
            } else {
                None
            };

            udts.last_mut()
                .expect("No UDTs found!")
                .members
                .push(udt::UdtMember {
                    name: name.clone(),
                    description: description.clone(),
                    data_type: data_type.clone(),
                    array_bounds: bounds.clone(),
                    external_write: external_write,
                    external_read: external_read,
                    hidden: false,
                    target: target.clone(),
                    bit_num: if data_type.to_uppercase() == "BOOL" && bounds == None {
                        Some(bit_num)
                    } else {
                        None
                    },
                });

            if let &Some(_) = &target {
                bit_num += 1;
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
