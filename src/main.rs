use chrono::Local;
use quick_xml;
use regex::RegexBuilder;
use std::env::args;
use std::io::Cursor;
use std::process::exit;
use std::{fs, vec};

#[allow(dead_code)]
struct UdtMember {
    name: String,
    description: Option<String>,
    data_type: String,
    array_bounds: Option<(isize, isize)>,
    external_read: bool,
    external_write: bool,
    hidden: bool,
    target: Option<String>,
    bit_num: Option<usize>,
}

#[allow(dead_code)]
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

// Check if data type should use decimal radix
fn numeric_type(inp: &str) -> bool {
    ["REAL",
    "SINT",
    "INT",
    "DINT",
    "LINT",
    "USINT",
    "UINT",
    "UDINT",
    "ULINT",
    "BOOL",
    "LREAL",
    "BIT"].contains(&inp.to_uppercase().as_str())
}

// Check if data type should use character radix
fn char_type(inp: &str) -> bool {
    inp.to_uppercase() == "CHAR" ||
    "STRING".contains(inp.to_uppercase().as_str())
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
            "-h" | "--help" => {
                println!("{}", help)
            }
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
    // TODO: Kill it with fire! 
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
        let mut target_num = 0;
        let mut bit_num = 0;
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
                        UdtMember {
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
                .push(UdtMember {
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
    let mut writer = quick_xml::Writer::new_with_indent(Cursor::new(Vec::<u8>::new()), b' ', 4);

    // Create description element
    let write_description = |description: &Option<String>,
                             writer: &mut quick_xml::Writer<Cursor<Vec<u8>>>|
     -> Result<_, quick_xml::Error> {
        if let Some(desc) = description {
            writer
                .create_element("Description")
                .write_cdata_content(quick_xml::events::BytesCData::new(desc))?;
        }
        Ok(())
    };

    // Create member elements for UDTs
    let write_members = |udt: &Udt,
                         writer: &mut quick_xml::Writer<Cursor<Vec<u8>>>|
     -> Result<_, quick_xml::Error> {
        for member in &udt.members {
            let bounds = (if let Some(dim) = member.array_bounds {
                if member.data_type.to_uppercase() == "BOOL" {
                    ((dim.1 + 1) as usize).div_ceil(32) * 32
                } else {
                    (dim.1 + 1) as usize
                }
            } else {
                0
            })
            .to_string();

            let data_type =
                if let (None, true) = (member.array_bounds, member.data_type.to_uppercase() == "BOOL") {
                    "BIT"
                } else {
                    member.data_type.as_str()
                };

            let hidden = member.hidden.to_string();

            let external_access = if member.external_write {
                "Read/Write"
            } else if member.external_read {
                "ReadOnly"
            } else {
                "None"
            };

            let bit_num = if let Some(bit) = member.bit_num {
                bit.to_string()
            } else {
                "".to_string()
            };

            let radix = if numeric_type(data_type) {
                "Decimal"
            } else if char_type(data_type) {
                "Char"
            } else {
                "NullType"
            }; 

            let mut attributes = vec![
                ("Name", member.name.as_str()),
                ("DataType", data_type),
                ("Dimensions", bounds.as_str()),
                ("Radix", radix),
                ("Hidden", hidden.as_str()),
                ("ExternalAccess", external_access),
            ];

            // (&Some(_), &Some(_)) = (&member.target, &member.bit_num)
            if let (None, true) = (member.array_bounds, member.data_type.to_uppercase() == "BOOL") {
                attributes.push(("Target", member.target.as_ref().unwrap().as_str()));
                attributes.push(("BitNumber", bit_num.as_str()))
            }

            writer
                .create_element("Member")
                .with_attributes(attributes)
                .write_inner_content(|writer| write_description(&member.description, writer))?;
        }
        Ok(())
    };

    // Create a data type elements
    let write_data_type = |udt: &Udt,
                           writer: &mut quick_xml::Writer<Cursor<Vec<u8>>>|
     -> Result<_, quick_xml::Error> {
        writer
            .create_element("DataType")
            .with_attributes([
                ("Use", "Target"),
                ("Name", &udt.name),
                ("Family", "NoFamily"),
                ("Class", "User"),
            ])
            .write_inner_content(|writer| {
                write_description(&udt.description, writer)?; 

                writer
                    .create_element("Members")
                    .write_inner_content(|writer| write_members(udt, writer))?; 
                Ok::<_, quick_xml::Error>(())
            })?;
        Ok(())
    };

    // Create dependancy elements
    let write_dependencies =
        |writer: &mut quick_xml::Writer<Cursor<Vec<u8>>>| -> Result<_, quick_xml::Error> {
            for udt in &udts {
                writer
                    .create_element("Dependency")
                    .with_attributes([("Type", "DataType"), ("Name", udt.name.as_str())])
                    .write_empty()?;
            }
            Ok(())
        };

    // Create elemnt for parent data type
    let write_parent_data_type =
        |writer: &mut quick_xml::Writer<Cursor<Vec<u8>>>| -> Result<_, quick_xml::Error> {
            writer
            .create_element("DataType")
            .with_attributes([
                ("Use", "Target"),
                ("Name", &parent_udt.name),
                ("Family", "NoFamily"),
                ("Class", "User"),
            ])
            .write_inner_content(|writer| {
                write_description(&parent_udt.description, writer)?; 

                writer
                    .create_element("Members")
                    .write_inner_content(|writer| {
                        write_members(&parent_udt, writer)
                    })?; 

                writer
                    .create_element("Dependencies")
                    .write_inner_content(write_dependencies)?;
                Ok::<_, quick_xml::Error>(())
            })?;
            Ok(())
        };

    // Create elements for all UDTs
    let write_all_data_types =
        |writer: &mut quick_xml::Writer<Cursor<Vec<u8>>>| -> Result<_, quick_xml::Error> {
            write_parent_data_type(writer)?;

            for udt in &udts {
                write_data_type(&udt, writer)?;
            }
            Ok(())
        };

    // Create data types element
    let write_data_types =
        |writer: &mut quick_xml::Writer<Cursor<Vec<u8>>>| -> Result<_, quick_xml::Error> {
            writer
                .create_element("DataTypes")
                .with_attribute(("Use", "Context"))
                .write_inner_content(write_all_data_types)?;
            Ok(())
        };

    // Create controller element
    let write_controller =
        |writer: &mut quick_xml::Writer<Cursor<Vec<u8>>>| -> Result<_, quick_xml::Error> {
            writer
                .create_element("Controller")
                .with_attributes([("Use", "Context"), ("Name", "UdtConverter")])
                .write_inner_content(write_data_types)?;
            Ok(())
        };

    // Create root element
    writer.create_element("RSLogix5000Content")
        .with_attributes([
            ("SchemaRevision", "1.0"), 
            ("SoftwareRevision", "35.0"), 
            ("TargetName", &parent_udt.name), 
            ("TargetType", "DataType"), 
            ("ContainsContext", "true"), 
            ("ExportData", &Local::now().format("%a %b %d %H:%M:%S %Y").to_string()), 
            ("ExportOptions", "References NoRawData L5KData DecoratedData Context Dependencies ForceProtectedEncoding AllProjDocTrans"), 
        ]).write_inner_content(write_controller)
    .unwrap();

    // let result = writer.into_inner().into_inner();
    fs::write("Data Types/dummy.L5X", writer.into_inner().into_inner()).unwrap();
}
