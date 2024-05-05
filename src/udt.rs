use regex::{Captures, Regex, RegexBuilder};

pub struct UdtMember {
    pub name: String,
    pub description: Option<String>,
    pub data_type: String,
    pub array_bounds: Option<(isize, isize)>,
    pub external_read: bool,
    pub external_write: bool,
    pub hidden: bool,
    pub target: Option<String>,
    pub bit_num: Option<usize>,
}

pub struct Udt {
    pub name: String,
    pub description: Option<String>,
    pub _version: String,
    pub members: Vec<UdtMember>,
}

/// Target numbers and bit numbers for bool member variables
pub struct BoolTargets {
    pub target_num: usize,
    pub bit_num: usize,
}

impl BoolTargets {
    /// Return new BoolTargets with values of 0
    pub fn new() -> BoolTargets {
        BoolTargets {
            target_num: 0,
            bit_num: 0,
        }
    }

    /// Increment bit_num, and reset it to 0 and increment target_num when it reaches 7
    pub fn inc(&mut self) {
        if self.bit_num >= 7 {
            self.bit_num = 0;
            self.target_num += 1;
        } else {
            self.bit_num += 1;
        }
    }
}

// TODO: add functionality to automate adding new string data types
/// Converts the syntax for custom length strings to a valid syntax for Rockwell.
/// However, custom length strings must be separately defined data types
pub fn reformat_string(input: &str) -> String {
    if let Some(_) = input.to_uppercase().find("STRING[") {
        let end = input.find("]").expect("Invalid STRING type format");
        let mut output = "STRING_".to_string();
        output.push_str(&input[7..end]);
        output.to_owned()
    } else {
        input.to_string()
    }
}

/// Checks if data type should use decimal radix
pub fn numeric_type(inp: &str) -> bool {
    [
        "REAL", "SINT", "INT", "DINT", "LINT", "USINT", "UINT", "UDINT", "ULINT", "BOOL", "LREAL",
        "BIT",
    ]
    .contains(&inp.to_uppercase().as_str())
}

/// Checks if data type should use character radix
pub fn char_type(inp: &str) -> bool {
    inp.to_uppercase() == "CHAR" || "STRING".contains(inp.to_uppercase().as_str())
}

// TODO: find a way to do this without multiple String allocations (fixed length strings maybe?)
/// Converts common elementary types from Siemens to an equivalent for Rockwell
pub fn convert_type(input: &str) -> String {
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

// These regex patterns were made at https://regex101.com/ using the Rust flavor
/// Regex pattern for parsing the head and body from exported UDTs from TIA Portal
pub fn build_udt_regex() -> Regex {
    RegexBuilder::new(r#"TYPE *"(?<udt_type>\S*)"\s*(?:TITLE *= *(?<udt_title>[\S\s]*?)\n)?(?:VERSION *: *(?<udt_version>[\s\S]*?)\n)[\s\S]*?STRUCT(?<udt_body>[\s\S]*?)END_STRUCT;?[\s\S]*?END_TYPE"#)
        .case_insensitive(true)
        .multi_line(true)
        .build()
        .expect("Invalid Regex pattern!")
}

// TODO: Kill it with fire!
/// Regex pattern for parsing member variables from the body of an exported UDT from TIA Portal
pub fn build_member_regex() -> Regex {
    RegexBuilder::new(r#"\s*"?(?<member_name>[a-z0-9_]*)"?\s*?(?:\{(?:\s*?ExternalAccessible\s*?:=\s*?'(?<ext_acs>[a-z]*?)';)?(?:\s*?ExternalVisible\s*?:=\s*?'(?<ext_vis>[a-z]*?)';)?(?:\s*?ExternalWritable\s*?:=\s*?'(?<ext_wrt>[a-z]*?)')?[\s\S]*?})?\s*?:\s*?(?:Array\[(?<bound_lower>[[:digit:]]+)\.\.(?<bound_upper>[[:digit:]])+\]\s*?of\s+?)?"?(?<member_type>[a-z1-9_]*)"?(?:\s*?:=\s*?[\s\S]*?)?;\s*?(?://(?<member_description>[\s\S]*?))?\n"#)
        .case_insensitive(true)
        .multi_line(true)
        .build()
        .expect("Invalid regex pattern!")
}

pub fn get_udt_description(udt_str: &Captures) -> Option<String> {
    if let Some(desc) = udt_str.name("udt_title") {
        Some(String::from(desc.as_str()))
    } else {
        None
    }
}

/// Get array bounds (if they exist) from the regex parser
pub fn get_bounds(member_str: &Captures) -> Option<(isize, isize)> {
    let (lower_bound, upper_bound) = (
        if let Some(bound) = member_str.name("bound_lower") {
            Some(
                String::from(bound.as_str())
                    .parse()
                    .expect("Lower bound invalid format"),
            )
        } else {
            None
        },
        if let Some(bound) = member_str.name("bound_upper") {
            Some(
                String::from(bound.as_str())
                    .parse()
                    .expect("Upper bound invalid format"),
            )
        } else {
            None
        },
    );

    if let (Some(lower), Some(upper)) = (lower_bound, upper_bound) {
        Some((lower, upper))
    } else {
        None
    }
}

/// Get description (if it exists) from the regex parser
pub fn get_member_description(member_str: &Captures) -> Option<String> {
    if let Some(desc) = member_str.name("member_description") {
        Some(String::from(desc.as_str()))
    } else {
        None
    }
}

/// Determine if member is externally writeable
pub fn external_write(member_str: &Captures) -> bool {
    if let Some(ext_wrt) = member_str.name("ext_wrt") {
        if ext_wrt.as_str().to_lowercase() == "false" {
            false
        } else {
            true
        }
    } else {
        true
    }
}

/// Determine if member is externally readable
pub fn external_read(member_str: &Captures) -> bool {
    if let Some(ext_vis) = member_str.name("ext_vis") {
        if ext_vis.as_str().to_lowercase() == "false" {
            false
        } else {
            true
        }
    } else {
        true
    }
}

/// Special case for inidividual bools to assign them to bits of hidden SINTs.
/// Also creates the hidden SINTs as needed and adds them to the UDT
pub fn get_target(
    member_str: &Captures,
    udts: &mut Vec<Udt>,
    target_nums: &BoolTargets,
) -> Option<String> {
    let data_type = convert_type(&member_str["member_type"]).to_uppercase();
    let mut target_name = "ZZZZZZZZZZ".to_string();
    target_name.push_str(&udts.last().unwrap().name);

    if let (true, None) = (data_type == "BOOL", get_bounds(member_str)) {
        target_name.push_str(&target_nums.target_num.to_string());

        if target_nums.bit_num == 0 {
            udts.last_mut().unwrap().members.insert(
                target_nums.target_num,
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

    if data_type.to_uppercase() == "BOOL" {
        Some(target_name)
    } else {
        None
    }
}
