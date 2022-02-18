use convert_case::{Case, Casing};
use regex::Regex;
use std::cell::RefCell;
use std::collections::HashMap;
use std::env;
use std::fmt::Debug;
use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::Path;
use std::sync::Mutex;

#[derive(Debug)]
struct OutputModule {
    content: Vec<OutputField>,
    enums: HashMap<String, Enum>,
    sub_modules: HashMap<String, OutputModule>,
}

impl OutputModule {
    fn new() -> OutputModule {
        OutputModule {
            content: Vec::new(),
            enums: HashMap::new(),
            sub_modules: HashMap::new(),
        }
    }
}

#[derive(Debug)]
struct Enum {
    values: Vec<String>,
}

#[derive(Debug)]
struct OutputField {
    field_name: String,
    field_type: String,
    required: bool,
}
enum ParsedField {
    Static {
        type_name: &'static str,
        pattern: StaticAnnotationPattern,
    },
    String {
        type_name: String,
        pattern: StaticAnnotationPattern,
    },
}

#[derive(Debug)]
struct StaticField {
    type_name: &'static str,
    pattern: StaticAnnotationPattern,
}
#[derive(Debug)]
struct StringField {
    type_name: String,
    pattern: StaticAnnotationPattern,
}
#[derive(Debug)]
enum StaticAnnotationPattern {
    Empty,
    Optional,
}

impl StaticAnnotationPattern {
    fn serde_options(&self, callback: &mut dyn FnMut(&str) -> ()) {
        match self {
            StaticAnnotationPattern::Empty => {}
            StaticAnnotationPattern::Optional => {
                callback("deserialize_with = \"deserialize_optional_from_string\"");
                callback("serialize_with = \"serialize_optional_to_string\"");
                callback("default");
            }
        }
    }
}

trait FieldSettings {
    fn field_name(&self) -> &str;
    fn serde_options(&self, callback: &mut dyn FnMut(&str) -> ());
}

impl FieldSettings for ParsedField {
    fn field_name(&self) -> &str {
        match self {
            ParsedField::Static { type_name, .. } => type_name,
            ParsedField::String { type_name, .. } => type_name.as_str(),
        }
    }

    fn serde_options(&self, callback: &mut dyn FnMut(&str) -> ()) {
        match self {
            ParsedField::Static { pattern, .. } => {
                pattern.serde_options(callback);
            }
            ParsedField::String { .. } => {}
        }
    }
}

impl FieldSettings for StaticField {
    fn field_name(&self) -> &str {
        self.type_name
    }

    fn serde_options(&self, callback: &mut dyn FnMut(&str) -> ()) {
        self.pattern.serde_options(callback);
    }
}

impl FieldSettings for StringField {
    fn field_name(&self) -> &str {
        self.type_name.as_str()
    }

    fn serde_options(&self, callback: &mut dyn FnMut(&str) -> ()) {
        self.pattern.serde_options(callback)
    }
}

fn main() -> std::io::Result<()> {
    let src_file = BufReader::new(File::open("src/routeros/model/bridge.txt")?);
    let mut line_iter = src_file.lines();
    let part_pattern = Regex::new("/([0-9a-z]+)").unwrap();
    let mut root_module = OutputModule::new();
    let mut open_module: Option<&mut OutputModule> = None;
    loop {
        match line_iter.next() {
            Some(Ok(line)) => {
                if line.starts_with("/") {
                    let mut current_module = &mut root_module;
                    for part in part_pattern.find_iter(line.as_str()) {
                        let match_str = part.as_str();
                        if match_str.len() < 2 {
                            continue;
                        }
                        let comp_name: String = match_str.chars().skip(1).collect();
                        current_module = current_module
                            .sub_modules
                            .entry(comp_name)
                            .or_insert(OutputModule::new());
                    }
                    open_module = Some(current_module);
                } else if let Some(current_module) = &mut open_module {
                    if let Some((field, optional_enum)) = parse_field_line(line) {
                        if let Some(e) = optional_enum {
                            current_module.enums.insert(field.field_type.clone(), e);
                        }
                        current_module.content.push(field);
                    }
                }
            }
            _ => break,
        }
    }
    println!("Tree: {:#?}", root_module);
    let mut current_struct: Box<String> = Box::new(str::to_string(""));

    let out_dir = env::var_os("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("generated.rs");
    let mut write_handle = BufWriter::new(File::create(dest_path)?);
    writeln!(write_handle, "pub mod generated{{")?;
    //writeln!(write_handle, "  pub mod model{{")?;
    let empty_path = vec![];
    dump_module(&mut write_handle, &root_module, 1, &empty_path, "")?;
    //writeln!(write_handle, "  }}")?;
    writeln!(write_handle, "}}")?;
    println!("cargo:rerun-if-changed=generated.rs");
    std::io::Result::Ok(())
}

fn expand_enum_name(name: &str) -> Option<String> {
    let camel_name = name.to_case(Case::UpperCamel);
    let mut char_iter = camel_name.chars();
    if let Some(ch) = char_iter.next() {
        if ch.is_alphabetic() {
            Some(camel_name)
        } else {
            Some(format!("_{camel_name}"))
        }
    } else {
        None
    }
}

fn dump_module(
    file: &mut BufWriter<File>,
    module_data: &OutputModule,
    depth: u8,
    parent_path: &Vec<&str>,
    module_name: &str,
) -> std::io::Result<()> {
    if module_data.sub_modules.is_empty() && module_data.content.is_empty() {
        return Ok(());
    }
    let mut module_path = parent_path.clone();
    if !module_name.is_empty() {
        module_path.push(module_name);
    }

    let prefix = "  ".repeat(depth.into());
    if !module_data.content.is_empty() {
        let model_name = module_path[1..].join("-").to_case(Case::UpperCamel);
        for (type_name, type_values) in module_data.enums.iter() {
            writeln!(file, "{prefix}#[derive(Serialize, Deserialize, Debug)]")?;
            writeln!(file, "{prefix}#[serde(rename_all = \"kebab-case\")]")?;
            writeln!(file, "{prefix}pub enum {type_name} {{")?;
            for value in type_values.values.iter() {
                if let Some(enum_value) = expand_enum_name(value.as_str()) {
                    if !enum_value.to_case(Case::Kebab).eq(value) {
                        writeln!(file, "{prefix}  #[serde(rename=\"{value}\")]")?;
                    }
                    writeln!(file, "{prefix}  {enum_value},")?;
                }
            }
            writeln!(file, "{prefix}}}")?;
        }

        writeln!(file, "{prefix}use serde::{{Deserialize, Serialize}};")?;
        writeln!(file, "{prefix}use crate::routeros::json::{{")?;
        writeln!(
            file,
            "{prefix}    deserialize_number_ranges_from_string, deserialize_optional_from_string,"
        )?;
        writeln!(file, "{prefix}    serialize_number_ranges_to_string, serialize_optional_to_string, serialize_stringset_to_string,")?;
        writeln!(file, "{prefix}    stringset_from_string,")?;
        writeln!(file, "{prefix}}};")?;
        writeln!(file, "{prefix}#[derive(Serialize, Deserialize, Debug)]")?;
        writeln!(file, "{prefix}#[serde(rename_all = \"kebab-case\")]")?;
        writeln!(file, "{prefix}pub struct {model_name} {{")?;
        for field in module_data.content.iter() {
            let mut field_name = String::new();
            let mut last_was_masked = true;
            for ch in field.field_name.chars() {
                if ch.is_alphanumeric() {
                    field_name.push(ch);
                    last_was_masked = false;
                } else {
                    if !last_was_masked {
                        field_name.push('_')
                    };
                    last_was_masked = true;
                }
            }
            let field_settings: Option<Box<dyn FieldSettings>> = if field.required {
                if let Some(_) = module_data.enums.get(field.field_type.as_str()) {
                    Some(Box::new(StringField {
                        type_name: format!("Option<{}>", field.field_type),
                        pattern: StaticAnnotationPattern::Empty,
                    }))
                } else {
                    match field.field_type.as_str() {
                        "string" => Some(Box::new(StaticField {
                            type_name: "String",
                            pattern: StaticAnnotationPattern::Empty,
                        })),
                        "u16" => Some(Box::new(StaticField {
                            type_name: "u16",
                            pattern: StaticAnnotationPattern::Empty,
                        })),
                        "boolean" => Some(Box::new(StaticField {
                            type_name: "bool",
                            pattern: StaticAnnotationPattern::Empty,
                        })),
                        _ => None,
                    }
                }
            } else {
                if let Some(_) = module_data.enums.get(field.field_type.as_str()) {
                    Some(Box::new(StringField {
                        type_name: format!("Option<{}>", field.field_type),
                        pattern: StaticAnnotationPattern::Empty,
                    }))
                } else {
                    println!("Field type: {}", field.field_type);
                    match field.field_type.as_str() {
                        "boolean" => Some(Box::new(StaticField {
                            type_name: "Option<bool>",
                            pattern: StaticAnnotationPattern::Optional,
                        })),
                        "string" => Some(Box::new(StaticField {
                            type_name: "Option<String>",
                            pattern: StaticAnnotationPattern::Empty,
                        })),
                        "u16" => Some(Box::new(StaticField {
                            type_name: "Option<u16>",
                            pattern: StaticAnnotationPattern::Optional,
                        })),
                        _ => None,
                    }
                }
            };
            if let Some(field_settings) = field_settings {
                let mut serde_settings = vec![];
                if !field_name.to_case(Case::Kebab).eq(&field.field_name) {
                    let string = format!(
                        "rename=\"{original_name}\"",
                        original_name = field.field_name
                    );
                    serde_settings.push(string);
                }

                field_settings.serde_options(&mut (|v: &str| serde_settings.push(v.into())));
                if !serde_settings.is_empty() {
                    writeln!(file, "{prefix}  #[serde(")?;
                    for setting in serde_settings {
                        writeln!(file, "{prefix}    {setting},")?
                    }
                    writeln!(file, "{prefix}  )]")?
                }
                writeln!(
                    file,
                    "{prefix}  {field_name}: {field_type},",
                    field_type = field_settings.field_name()
                )?;
            }
        }
        writeln!(file, "{prefix}}}")?;
        /*
        writeln!(file, " ")?;
        writeln!(
            file,
            "{prefix}impl crate::routeros::model::RouterOsResource for {model_name} {{"
        )?;
        writeln!(file, "{prefix}  fn resource_path() -> &'static str {{")?;
        writeln!(file, "{prefix}    \"{path}\"", path = module_path.join("/"))?;
        writeln!(file, "{prefix}  }}")?;
        writeln!(file, "{prefix}}}")?;

         */
    }
    for (module_name, module_data) in module_data.sub_modules.iter() {
        writeln!(file, "{prefix}pub mod {module_name} {{")?;
        dump_module(file, module_data, depth + 1, &module_path, module_name)?;
        writeln!(file, "{prefix}}}")?;
    }
    Ok(())
}

fn parse_field_line(line: String) -> Option<(OutputField, Option<Enum>)> {
    let mut char_iter = line.chars().into_iter();
    let mut field_name = String::new();
    loop {
        match char_iter.next() {
            None => break,
            Some(ch) if ch == ':' => break,
            Some(ch) => field_name.push(ch),
        }
    }
    let field_type = Mutex::new(RefCell::new(String::new()));
    let mut field_type_components: Vec<String> = vec![];
    let mut required = false;
    //let mut closure_field_type = field_type.clone();
    let mut push_to_components = || {
        let mut guard = field_type.lock().unwrap();
        let field_type = guard.get_mut();
        let striped_field_type = String::from(field_type.trim());
        if striped_field_type.len() > 0 {
            field_type_components.push(striped_field_type);
        }
        field_type.clear();
    };
    loop {
        match char_iter.next() {
            None => break,
            Some(ch) if ch == '!' => {
                required = true;
                break;
            }
            Some(ch) if ch == ',' => {
                push_to_components();
            }
            Some(ch) => field_type.lock().unwrap().get_mut().push(ch),
        }
    }
    push_to_components();

    let trimmed_name = field_name.trim();
    if trimmed_name.is_empty() {
        return None;
    }
    if field_type_components.is_empty() {
        return Some((
            OutputField {
                field_name: String::from(trimmed_name),
                field_type: String::from("string"),
                required: false,
            },
            None,
        ));
    }
    return if field_type_components.len() == 1 {
        let trimmed_type = field_type_components.remove(0);
        Some((
            OutputField {
                field_name: String::from(trimmed_name),
                field_type: trimmed_type,
                required,
            },
            None,
        ))
    } else {
        Some((
            OutputField {
                field_name: String::from(trimmed_name),
                field_type: trimmed_name.to_case(Case::UpperCamel),
                required,
            },
            Some(Enum {
                values: field_type_components,
            }),
        ))
    };
}
