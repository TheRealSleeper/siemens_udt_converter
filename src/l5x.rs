use crate::udt;
use chrono::Local;
use quick_xml;
use std::{io::Cursor, vec};

/// Create description element
fn write_description(
    description: &Option<String>,
    writer: &mut quick_xml::Writer<Cursor<Vec<u8>>>,
) -> Result<(), quick_xml::Error> {
    if let Some(desc) = description {
        writer
            .create_element("Description")
            .write_cdata_content(quick_xml::events::BytesCData::new(desc))?;
    }
    Ok(())
}

/// Write members to UDT element
fn write_members(
    udt: &udt::Udt,
    writer: &mut quick_xml::Writer<Cursor<Vec<u8>>>,
) -> Result<(), quick_xml::Error> {
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

        let data_type = if let (None, true) = (
            member.array_bounds,
            member.data_type.to_uppercase() == "BOOL",
        ) {
            "BIT"
        } else {
            member.data_type.as_str()
        };

        let hidden = member.hidden.to_string();

        let external_access = if member.external_write {
            "Read/Write"
        } else if member.external_read {
            "Read Only"
        } else {
            "None"
        };

        let bit_num = if let Some(bit) = member.bit_num {
            bit.to_string()
        } else {
            "".to_string()
        };

        let radix = if udt::numeric_type(data_type) {
            "Decimal"
        } else if udt::char_type(data_type) {
            "Char"
        } else {
            "NullType"
        };

        let mut attributes = vec![
            ("Name", member.name.as_str()),
            ("DataType", data_type),
            ("Dimension", bounds.as_str()),
            ("Radix", radix),
            ("Hidden", hidden.as_str()),
            ("ExternalAccess", external_access),
        ];

        if let (None, true) = (
            member.array_bounds,
            member.data_type.to_uppercase() == "BOOL",
        ) {
            attributes.push(("Target", member.target.as_ref().unwrap().as_str()));
            attributes.push(("BitNumber", bit_num.as_str()))
        }

        writer
            .create_element("Member")
            .with_attributes(attributes)
            .write_inner_content(|writer| write_description(&member.description, writer))?;
    }
    Ok(())
}

/// Create a data type elements
fn write_data_type(
    udt: &udt::Udt,
    writer: &mut quick_xml::Writer<Cursor<Vec<u8>>>,
) -> Result<(), quick_xml::Error> {
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
}

/// Create dependancy elements
fn write_dependencies(
    udts: &Vec<udt::Udt>,
    writer: &mut quick_xml::Writer<Cursor<Vec<u8>>>,
) -> Result<(), quick_xml::Error> {
    for udt in udts {
        writer
            .create_element("Dependency")
            .with_attributes([("Type", "DataType"), ("Name", udt.name.as_str())])
            .write_empty()?;
    }
    Ok(())
}

/// Create elemnt for parent data type
fn write_parent_data_type(
    udts: &Vec<udt::Udt>,
    parent_udt: udt::Udt,
    writer: &mut quick_xml::Writer<Cursor<Vec<u8>>>,
) -> Result<(), quick_xml::Error> {
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
                .write_inner_content(|writer| write_members(&parent_udt, writer))?;

            writer
                .create_element("Dependencies")
                .write_inner_content(|writer| write_dependencies(udts, writer))?;
            Ok::<_, quick_xml::Error>(())
        })?;
    Ok(())
}

/// Create elements for all UDTs
fn write_all_data_types(
    udts: &Vec<udt::Udt>,
    parent_udt: udt::Udt,
    writer: &mut quick_xml::Writer<Cursor<Vec<u8>>>,
) -> Result<(), quick_xml::Error> {
    write_parent_data_type(udts, parent_udt, writer)?;

    for udt in udts {
        write_data_type(&udt, writer)?;
    }
    Ok(())
}

/// Create data types element
fn write_data_types(
    udts: &Vec<udt::Udt>,
    parent_udt: udt::Udt,
    writer: &mut quick_xml::Writer<Cursor<Vec<u8>>>,
) -> Result<(), quick_xml::Error> {
    writer
        .create_element("DataTypes")
        .with_attribute(("Use", "Context"))
        .write_inner_content(|writer| write_all_data_types(udts, parent_udt, writer))?;
    Ok(())
}

/// Create controller element
fn write_controller(
    udts: &Vec<udt::Udt>,
    parent_udt: udt::Udt,
    writer: &mut quick_xml::Writer<Cursor<Vec<u8>>>,
) -> Result<(), quick_xml::Error> {
    writer
        .create_element("Controller")
        .with_attributes([("Use", "Context"), ("Name", "UdtConverter")])
        .write_inner_content(|writer| write_data_types(udts, parent_udt, writer))?;
    Ok(())
}

/// Generates L5X file (stored in memory as Vec<u8>) for parsed UDTs
pub fn create_l5x(
    udts: &Vec<udt::Udt>,
    parent_udt: udt::Udt,
) -> Result<quick_xml::Writer<Cursor<Vec<u8>>>, quick_xml::Error> {
    let mut writer = quick_xml::Writer::new_with_indent(Cursor::new(Vec::<u8>::new()), b' ', 4);

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
        ]).write_inner_content(|writer| {
            write_controller(udts, parent_udt, writer)
        })?;

    Ok(writer)
}
