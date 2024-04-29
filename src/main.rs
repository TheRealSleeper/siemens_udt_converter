use std::env::args;
use std::fs;
use std::process::exit;
use regex::RegexBuilder;
use std::rc::Rc;
use chrono::Local; 
use quick_xml; 
use std::io::Cursor; 

struct UdtMember {
    _name: Rc<str>,
    _description: Option<Rc<str>>,
    _data_type: Rc<str>,
    _array_bounds: Option<(isize, isize)>,
    _external_read: bool, 
    _external_write: bool, 
}

struct Udt {
    _name: Rc<str>,
    _description: Option<Rc<str>>,
    _version: Rc<str>,
    members: Vec<UdtMember>,
}

// Converts the syntax for custom length strings to a valid syntax for Rockwell
// However, custom length strings must be separately defined data types
// TODO: add functionality to automate that
fn reformat_string(input: &str) -> String {
    if let Some(_) = input.to_uppercase().find("STRING[") {
        let end = input.find("]").expect("Invalid STRING type format");
        let mut output = "STRING_".to_string();
        output.push_str(&input[7..end]);
        output.to_owned()
    } else {
        input.to_string()
    }
}

// TODO: find a way to do this without multiple String allocations (fixed length strings maybe?)
fn convert_type(input: &str) -> String {
    match input.to_uppercase().as_str() {
        "BYTE" => "USINT".to_string(),
        "WORD" => "UINT".to_string(),
        "DWORD" => "UDINT".to_string(),
        "LWORD" => "ULINT".to_string(),
        "TIME" => "DINT".to_string(),
        "SINT" => "SINT".to_string(),
        "INT" => "INT".to_string(),
        "DINT" => "DINT".to_string(),
        "LINT" => "LINT".to_string(),
        "USINT" => "USINT".to_string(),
        "UINT" => "UINT".to_string(),
        "UDINT" => "UDINT".to_string(),
        "ULINT" => "ULINT".to_string(),
        "REAL" => "REAL".to_string(),
        "LREAL" => "LREAL".to_string(),
        "STRING" => "STRING".to_string(),
        "CHAR" => "CHAR".to_string(),
        "DTL" => "LDT".to_string(),
        &_ => reformat_string(input),
    }
}


// TODO: Handle BOOLs properly (in Rockwell changes to BIT and is assigned to a bit from a hidden SINT), except arrays which will all be BOOL[#] where # is a multiple of 32
fn main() {
    let mut env_args = args().skip(1); // Skip the first argument, which will always be the program name
    let mut input_path: Option<String> = None;
    let mut _output_path: Option<String> = None;

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
                _output_path = Some(env_args.next().expect("No argument given for output path!"))
            }
            "-h" | "--help" => println!("{}", help),
            _ => {
                println!("Unknown command");
                exit(-1);
            }
        }
    }

    // These regex patterns were made at https://regex101.com/ using the Rust flavor
    // Regex pattern for parsing the UDTs
    let udt_regex = RegexBuilder::new(r#"TYPE *"(?<udt_type>\S*)"\s*(?:TITLE *= *(?<udt_title>[\S\s]*?)\n)?(?:VERSION *: *(?<udt_version>[\s\S]*?)\n)[\s\S]*?STRUCT(?<udt_body>[\s\S]*?)END_STRUCT;?[\s\S]*?END_TYPE"#)
        .case_insensitive(true)
        .multi_line(true)
        .build()
        .expect("Invalid Regex pattern!");

    // Regex pattern for parsing member variables of a UDT, declared before looping to avoid rebuilding Regex with each pass
    // TODO: Kill it with fire! "nom" seems to be a suggested alternative to Regex
    let member_regex = RegexBuilder::new(r#"\s*"?(?<member_name>[a-z1-9_]*)"?\s*?(?:\{(?:\s*?ExternalAccessible\s*?:=\s*?'(?<ext_acs>[a-z]*?)';)?(?:\s*?ExternalVisible\s*?:=\s*?'(?<ext_vis>[a-z]*?)';)?(?:\s*?ExternalWritable\s*?:=\s*?'(?<ext_wrt>[a-z]*?)')?[\s\S]*?})?\s*?:\s*?(?:Array\[(?<bound_lower>[[:digit:]]+)\.\.(?<bound_upper>[[:digit:]])+\]\s*?of\s+?)?"?(?<member_type>[a-z1-9_]*)"?(?:\s*?:=\s*?[\s\S]*?)?;\s*?(?://(?<member_description>[\s\S]*?))?\n"#)
        .case_insensitive(true)
        .multi_line(true)
        .build()
        .expect("Invalid regex pattern!");

    // Try to open specified input file
    let input = if let Some(path) = input_path {
        fs::read_to_string(path).expect("Invalid input path!")
    } else {
        panic!("No input file specified!");
    };

    // Generate empty Vec to be filled with parsed UDTs
    let mut udts: Vec<Udt> = vec![];

    // Separate input into individual UDTs
    let udts_str = udt_regex.captures_iter(&input);
    for udt_str in udts_str {
        let name: Rc<str> = udt_str["udt_type"].into();
        let description: Option<Rc<str>> = if let Some(desc) = udt_str.name("udt_title") {
            Some(Rc::from(desc.as_str()))
        } else {
            None
        };
        let version: Rc<str> = udt_str["udt_version"].into();
        let body: Rc<str> = udt_str["udt_body"].into();

        // Write
        udts.push(Udt {
            _name: name.clone(),
            _description: description.clone(),
            _version: version.clone(),
            members: vec![],
        });

        //Parse members in UDT body
        let members_str = member_regex.captures_iter(&body);
        for member_str in members_str {
            let name: Rc<str> = member_str["member_name"].into();
            let data_type: Rc<str> = convert_type(&member_str["member_type"]).into();

            let lower_bound: Option<Rc<str>> = if let Some(bound) = member_str.name("bound_lower") {
                Some(Rc::from(bound.as_str()))
            } else {
                None
            };
            let upper_bound: Option<Rc<str>> = if let Some(bound) = member_str.name("bound_upper") {
                Some(Rc::from(bound.as_str()))
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

            let description: Option<Rc<str>> =
                if let Some(desc) = member_str.name("member_description") {
                    Some(Rc::from(desc.as_str()))
                } else {
                    None
                };

            udts.last_mut()
                .expect("No UDTs found!")
                .members
                .push(UdtMember {
                    _name: name.clone(),
                    _description: description.clone(),
                    _data_type: data_type.clone(),
                    _array_bounds: bounds.clone(),
                    _external_write: true, 
                    _external_read: true,
                });
        }
    }

    // // TEMPORARY -- prints UDTs to console to verify correct parsing
    // for udt in udts {
    //     print!("{} ", udt.name);
    //     if let Some(desc) = udt.description {
    //         print!("// {}", desc);
    //     }
    //     print!("\n");

    //     for member in udt.members {
    //         print!("    {} : ", member.name);
    //         if let Some(bounds) = member.array_bounds {
    //             print!("Array[{}..{}] of ", bounds.0, bounds.1);
    //         }
    //         print!("{}; ", member.data_type);
    //         if let Some(description) = member.description {
    //             print!("// {}", description);
    //         }
    //         print!("\n");
    //     }

    //     print!("\n");
    // }


    // let mut buf = Vec::<u8>::new();
    let name = udts.last().unwrap()._name.to_string(); 
    let mut buf = Vec::<u8>::new(); 

    let root = quick_xml::Writer::new_with_indent(Cursor::new(buf), b' ', 4)
        .create_element("RSLogix5000Content")
        .with_attributes([
            ("SchemaRevision", "1.0"), 
            ("SoftwareRevision", "35.0"), 
            ("TargetName", &name), 
            ("TargetType", "DataType"), 
            ("ContainsContext", "true"), 
            ("ExportData", &chrono::Local::now().format("%a %b %d %H:%M:%S %Y").to_string()), 
            ("ExportOptions", "References NoRawData L5KData DecoratedData Context Dependencies ForceProtectedEncoding AllProjDocTrans"), 
        ]); 
        // .write_inner_content(|writer| {
        //     Err(writer.create_element("Controller")
        //         .with_attributes([
        //             ("Use", "Context"), 
        //             ("Name", "UdtConverter"), 
        //         ])
        //         .write_inner_content(|writer| {
        //             Err(writer.create_element("DataTypes")
        //                 .with_attribute(("Use", "Context"))
        //                 .write_inner_content(|writer| {
        //                     Err(writer.create_element("DataType")
        //                         .with_attributes([
        //                             ("Use", "Target"), 
        //                             ("Name", &name), 
        //                             ("Family", "NoFamily"), 
        //                             ("Class", "User")
        //                         ]))
        //                 }))
        //         }))
        // }); 

        println!("{}", quick_xml::Writer::); 
}
