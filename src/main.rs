use chrono::Local;
use quick_xml;
use regex::RegexBuilder;
use std::env::args;
use std::fs;
use std::io::Cursor;
use std::process::exit;
// use std::rc::Rc;

struct UdtMember {
    name: String,
    description: Option<String>,
    data_type: String,
    _array_bounds: Option<(isize, isize)>,
    _external_read: bool,
    _external_write: bool,
}

struct Udt {
    name: String,
    description: Option<String>,
    _version: String,
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
        let name: String = udt_str["udt_type"].into();
        let description: Option<String> = if let Some(desc) = udt_str.name("udt_title") {
            Some(String::from(desc.as_str()))
        } else {
            None
        };
        let version: String = udt_str["udt_version"].into();
        let body: String = udt_str["udt_body"].into();

        // Write
        udts.push(Udt {
            name: name.clone(),
            description: description.clone(),
            _version: version.clone(),
            members: vec![],
        });

        //Parse members in UDT body
        let members_str = member_regex.captures_iter(&body);
        for member_str in members_str {
            let name: String = member_str["member_name"].into();
            let data_type: String = convert_type(&member_str["member_type"]).into();

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

            udts.last_mut()
                .expect("No UDTs found!")
                .members
                .push(UdtMember {
                    name: name.clone(),
                    description: description.clone(),
                    data_type: data_type.clone(),
                    _array_bounds: bounds.clone(),
                    _external_write: true,
                    _external_read: true,
                });
        }
    }

    // #[allow(unused_mut, unused_variables)]
    let mut writer = quick_xml::Writer::new_with_indent(Cursor::new(Vec::<u8>::new()), b' ', 4);

    // Generate root
    let parent_udt = udts.pop().unwrap();
    writer.create_element("RSLogix5000Content")
        .with_attributes([
            ("SchemaRevision", "1.0"), 
            ("SoftwareRevision", "35.0"), 
            ("TargetName", &parent_udt.name), 
            ("TargetType", "DataType"), 
            ("ContainsContext", "true"), 
            ("ExportData", &Local::now().format("%a %b %d %H:%M:%S %Y").to_string()), 
            ("ExportOptions", "References NoRawData L5KData DecoratedData Context Dependencies ForceProtectedEncoding AllProjDocTrans"), 
        ])
        .write_inner_content::<_, quick_xml::Error>(|writer| {
            writer.create_element("Controller")
                .with_attributes([
                    ("Use", "Context"), 
                    ("Name", "UdtConverter"), 
                ])
                .write_inner_content::<_, quick_xml::Error>(|writer| {
                    writer.create_element("DataTypes")
                        .with_attribute(("Use", "Context"))
                        .write_inner_content::<_, quick_xml::Error>(|writer| {
                            writer.create_element("DataType")
                                .with_attributes([
                                    ("Use", "Target"), 
                                    ("Name", &parent_udt.name), 
                                    ("Family", "NoFamily"), 
                                    ("Class", "User")
                                ])
                                .write_inner_content::<_, quick_xml::Error>(|writer| {
                                    if let Some(desc) = parent_udt.description {
                                        writer.create_element("Description")
                                            .write_cdata_content(quick_xml::events::BytesCData::new(desc))?;
                                    }
                                    writer.create_element("members")
                                        .write_inner_content::<_, quick_xml::Error>(|writer| {
                                            for member in parent_udt.members {
                                                writer.create_element("Member")
                                                .with_attributes([
                                                    ("Name", member.name.as_str()),
                                                    ("DataType", member.data_type.as_str()), 
                                                    ("Radix", "NullType"),
                                                    ("Hidden", "false"),
                                                    ("ExternalAccess", "Read/Write"),
                                                ])
                                                .write_inner_content::<_, quick_xml::Error>(|writer| {
                                                    if let Some(desc) = member.description {
                                                        writer.create_element("Description")
                                                            .write_cdata_content(quick_xml::events::BytesCData::new(desc))?;
                                                    }
                                                    Ok(())
                                                })?;
                                            }
                                            Ok(())
                                        })?;
                                    Ok(())
                                })?;
                            Ok(())
                        })?;
                    Ok(())
                })?;
            Ok(())
        })
        .unwrap();

    // let result = writer.into_inner().into_inner();
    fs::write("Data Types/dummy.xml", writer.into_inner().into_inner()).unwrap();

    // println!("{}", String::from_utf8(result).unwrap());

    // writer.create_element("tag")
    // // We need to provide error type, because it is not named somewhere explicitly
    // .write_inner_content::<_, quick_xml::Error>(|writer| {
    //     let fruits = ["apple", "orange"];
    //     for (quant, item) in fruits.iter().enumerate() {
    //         writer
    //             .create_element("fruit")
    //             .with_attribute(("quantity", quant.to_string().as_str()))
    //             .write_text_content(quick_xml::events::BytesText::new(item))?;
    //     }
    //     Ok(())
    // });
}
