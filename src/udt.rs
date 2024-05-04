use regex::{Regex, RegexBuilder};

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

// Converts the syntax for custom length strings to a valid syntax for Rockwell
// However, custom length strings must be separately defined data types
// TODO: add functionality to automate that
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

// Check if data type should use decimal radix
pub fn numeric_type(inp: &str) -> bool {
    [
        "REAL", "SINT", "INT", "DINT", "LINT", "USINT", "UINT", "UDINT", "ULINT", "BOOL", "LREAL",
        "BIT",
    ]
    .contains(&inp.to_uppercase().as_str())
}

// Check if data type should use character radix
pub fn char_type(inp: &str) -> bool {
    inp.to_uppercase() == "CHAR" || "STRING".contains(inp.to_uppercase().as_str())
}

// TODO: find a way to do this without multiple String allocations (fixed length strings maybe?)
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
