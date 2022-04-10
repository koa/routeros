use convert_case::{Case, Casing};
use regex::Regex;
use std::cell::RefCell;
use std::collections::HashMap;
use std::ffi::OsStr;
use std::fmt::Debug;
use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::Path;
use std::sync::Mutex;
use std::{env, fs};

#[derive(Debug)]
struct OutputModule {
    content: Vec<OutputField>,
    enums: HashMap<String, Enum>,
    sub_modules: HashMap<String, OutputModule>,
    single_value: bool,
}

impl OutputModule {
    fn new() -> OutputModule {
        OutputModule {
            content: Vec::new(),
            enums: HashMap::new(),
            sub_modules: HashMap::new(),
            single_value: false,
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
    id: bool,
    read_only: bool,
}

fn main() -> std::io::Result<()> {
    let part_pattern = Regex::new("/([0-9a-z-]+)").unwrap();
    let mut root_module = OutputModule::new();
    let mut open_module: Option<&mut OutputModule> = None;
    let paths = fs::read_dir("ros_model")?;
    let input_file_extension = Option::Some("txt");
    for src_filename in paths
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|f| input_file_extension.eq(&f.extension().and_then(OsStr::to_str)))
    {
        let src_file = BufReader::new(File::open(&src_filename)?);
        let mut line_iter = src_file.lines();
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
                    } else if line.starts_with("1/") {
                        let mut current_module = &mut root_module;
                        for part in part_pattern.find_iter(&line.as_str()[1..]) {
                            let match_str = part.as_str();
                            if match_str.len() < 2 {
                                continue;
                            }
                            let comp_name: String = match_str.chars().skip(1).collect();
                            current_module = current_module
                                .sub_modules
                                .entry(comp_name)
                                .or_insert(OutputModule::new());
                            current_module.single_value = true;
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
    }

    let out_dir = env::var_os("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("generated.rs");
    let mut write_handle = BufWriter::new(File::create(dest_path)?);
    writeln!(write_handle, "#[allow(unused_imports)]")?;
    writeln!(write_handle, "pub mod generated{{")?;

    /*
        writeln!(write_handle, "  #[derive(Debug)]")?;
        writeln!(write_handle, "  pub enum Type{{")?;
        create_storage(&mut write_handle, &root_module, &vec![], "")?;
        writeln!(write_handle, "  }}")?;
    */
    dump_module(&mut write_handle, &root_module, 1, &vec![], "")?;
    writeln!(write_handle, "}}")?;
    println!("cargo:rerun-if-changed=generated.rs");
    std::io::Result::Ok(())
}

fn create_storage(
    file: &mut BufWriter<File>,
    module_data: &OutputModule,
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
    if !module_data.content.is_empty() {
        let model_name = module_path[1..].join("-").to_case(Case::UpperCamel);
        //let field_name = module_path[1..].join("-").to_case(Case::Snake);
        writeln!(file, "    {model_name},",)?;
    }
    for (module_name, module_data) in module_data.sub_modules.iter() {
        create_storage(file, module_data, &module_path, module_name)?;
    }
    Ok(())
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
        //writeln!(file, "{prefix}use ros_macro::RouterOsApiFieldAccess;")?;
        writeln!(file, "{prefix}use crate::routeros::client::api::RosError;")?;
        writeln!(
            file,
            "{prefix}use crate::routeros::model::{{Auto, Duration, IpNetAddr, ValueFormat, FieldDescription}};"
        )?;
        writeln!(file, "{prefix}use mac_address::MacAddress;")?;
        writeln!(file, "{prefix}use std::collections::HashSet;")?;
        writeln!(file, "{prefix}use std::ops::RangeInclusive;")?;
        let model_name = module_path[1..].join("-").to_case(Case::UpperCamel);
        for (type_name, type_values) in module_data.enums.iter() {
            writeln!(file, "{prefix}#[derive(Debug, Eq, PartialEq, Clone, Hash)]")?;
            writeln!(file, "{prefix}pub enum {type_name} {{")?;
            for value in type_values.values.iter() {
                if let Some(enum_value) = expand_enum_name(value.as_str()) {
                    writeln!(file, "{prefix}  {enum_value},")?;
                }
            }
            writeln!(file, "{prefix}}}")?;
            let default_value =
                expand_enum_name(type_values.values.iter().next().unwrap().as_str()).unwrap();
            writeln!(
                file,
                "{prefix}impl crate::routeros::model::RosValue for {type_name} {{"
            )?;
            writeln!(file, "{prefix}  type Type = {type_name};")?;
            writeln!(file, "{prefix}  type Err = RosError;")?;
            /*writeln!(
                file,
                "{prefix}  fn empty() -> Self::Type {{ {type_name}::{default_value} }}"
            )?;*/
            writeln!(
                file,
                "{prefix}  fn from_api(value: &str) -> Result<Self::Type, Self::Err> {{"
            )?;
            writeln!(file, "{prefix}    match value {{")?;
            for value in type_values.values.iter() {
                if let Some(enum_value) = expand_enum_name(value.as_str()) {
                    writeln!(
                        file,
                        "{prefix}      \"{value}\" => Ok({type_name}::{enum_value}),"
                    )?;
                }
            }
            writeln!(file, "{prefix}      unknown => Err(RosError::SimpleMessage(format!(\"unknown enum value: {{unknown}}\")))")?;
            writeln!(file, "{prefix}    }}")?;
            writeln!(file, "{prefix}  }}")?;
            writeln!(
                file,
                "{prefix}  fn to_api(&self,_:&ValueFormat) -> String {{"
            )?;
            writeln!(file, "{prefix}    String::from(match self {{")?;
            for value in type_values.values.iter() {
                if let Some(enum_value) = expand_enum_name(value.as_str()) {
                    writeln!(
                        file,
                        "{prefix}       {type_name}::{enum_value} => \"{value}\","
                    )?;
                }
            }
            writeln!(file, "{prefix}    }})")?;
            writeln!(file, "{prefix}  }}")?;
            writeln!(file, "{prefix}}}")?;
            writeln!(file, "{prefix}impl Default for {type_name} {{")?;
            writeln!(file, "{prefix}  fn default() -> Self {{")?;
            writeln!(file, "{prefix}    {type_name}::{default_value}")?;
            writeln!(file, "{prefix}  }}")?;
            writeln!(file, "{prefix}}}")?;
        }

        for field in module_data.content.iter() {
            let fd_name = field_description_name(&field.field_name);
            writeln!(file, "{prefix}const {fd_name}:crate::routeros::model::FieldDescription=crate::routeros::model::FieldDescription{{")?;
            writeln!(file, "{prefix}  name:\"{}\",", field.field_name)?;
            writeln!(file, "{prefix}  is_read_only:{},", field.read_only)?;
            writeln!(file, "{prefix}  is_id:{},", field.id)?;
            writeln!(file, "{prefix}}};")?;
        }
        writeln!(file, "{prefix}#[derive(Debug, Default, Clone)]")?;
        writeln!(file, "{prefix}pub struct {model_name} {{")?;
        //let mut has_id = false;
        for field in module_data.content.iter() {
            let field_name = expand_field_name(&field.field_name);
            let is_id = field.field_name == ".id";
            //has_id |= is_id;
            let access = if is_id { "" } else { "pub " };
            writeln!(
                file,
                "{prefix}  {access}{field_name}: crate::routeros::model::RosFieldValue<{field_type}>,",
                field_type = field.field_type
            )?;
        }
        writeln!(file, "{prefix}}}")?;
        let field_name = module_path[1..].join("-").to_case(Case::Snake);
        let module_path = module_path.join("/");
        writeln!(
            file,
            "{prefix}impl crate::routeros::model::RouterOsResource for {model_name} {{"
        )?;
        writeln!(file, "{prefix}   fn resource_path() -> &'static str {{")?;
        writeln!(file, "{prefix}     \"{module_path}\"")?;
        writeln!(file, "{prefix}    }}")?;

        writeln!(file, "{prefix}}}")?;

        writeln!(
            file,
            "{prefix}impl crate::routeros::model::{variant} for {model_name} {{",
            variant = if module_data.single_value {
                "RouterOsSingleResource"
            } else {
                "RouterOsListResource"
            }
        )?;
        writeln!(file, "{prefix}}}")?;

        writeln!(
            file,
            "{prefix}impl crate::routeros::model::RouterOsApiFieldAccess for {model_name} {{"
        )?;
        writeln!(file, "{prefix}  fn fields_mut(&mut self) -> Box<dyn Iterator<Item = (&'static crate::routeros::model::FieldDescription, &mut dyn crate::routeros::model::RosFieldAccessor)> + '_> {{")?;

        writeln!(
            file,
            "{prefix}    let fields: Vec<(&'static crate::routeros::model::FieldDescription, &mut dyn crate::routeros::model::RosFieldAccessor)> = vec!["
        )?;
        for field in module_data.content.iter() {
            let field_name_rust = expand_field_name(&field.field_name);
            let fd_name = field_description_name(&field.field_name);
            writeln!(
                file,
                "{prefix}      (&{fd_name}, &mut self.{field_name_rust}),"
            )?;
        }
        writeln!(file, "{prefix}    ];")?;
        writeln!(file, "{prefix}    Box::new(fields.into_iter())")?;
        writeln!(file, "{prefix}  }}")?;

        writeln!(file, "{prefix}  fn fields(&self) -> Box<dyn Iterator<Item = (&'static crate::routeros::model::FieldDescription, &dyn crate::routeros::model::RosFieldAccessor)> + '_> {{")?;

        writeln!(
            file,
            "{prefix}    let fields: Vec<(&'static crate::routeros::model::FieldDescription, &dyn crate::routeros::model::RosFieldAccessor)> = vec!["
        )?;
        for field in module_data.content.iter() {
            let field_name_rust = expand_field_name(&field.field_name);
            let fd_name = field_description_name(&field.field_name);
            writeln!(file, "{prefix}      (&{fd_name}, &self.{field_name_rust}),")?;
        }
        writeln!(file, "{prefix}   ];")?;
        writeln!(file, "{prefix}   Box::new(fields.into_iter())")?;
        writeln!(file, "{prefix}  }}")?;

        if let Some(_) = module_data
            .content
            .iter()
            .find(|t| &t.field_name == "dynamic")
        {
            writeln!(file, "{prefix}  fn is_dynamic(&self) -> bool {{")?;
            writeln!(file, "{prefix}   self.dynamic.get().unwrap_or(false)")?;
            writeln!(file, "{prefix}  }}")?;
        }

        writeln!(file, "{prefix}}}")?;
    }
    for (module_name, module_data) in module_data.sub_modules.iter() {
        writeln!(
            file,
            "{prefix}pub mod {module_name} {{",
            module_name = expand_field_name(module_name)
        )?;
        dump_module(file, module_data, depth + 1, &module_path, module_name)?;
        writeln!(file, "{prefix}}}")?;
    }
    Ok(())
}

fn expand_enum_name(name: &str) -> Option<String> {
    Some(name2rust(name, true)).filter(|v| !v.is_empty())
}
fn expand_field_name(name: &str) -> String {
    name2rust(name, false).to_case(Case::Snake)
}

fn name2rust(string: &str, start_capital: bool) -> String {
    if string.chars().next() == Some('.') {
        return name2rust(&string[1..], start_capital);
    }
    let mut result = String::new();
    let mut last_skipped = false;
    for ch in string.chars() {
        if ch.is_alphabetic() {
            if last_skipped || (start_capital && result.is_empty()) {
                result.push(ch.to_ascii_uppercase());
            } else {
                result.push(ch.to_ascii_lowercase());
            }
            last_skipped = false;
        } else if ch.is_digit(10) {
            if last_skipped || result.is_empty() {
                result.push('_');
            }
            result.push(ch);
            last_skipped = false;
        } else {
            last_skipped = true;
        }
    }
    result
}

fn parse_field_line(line: String) -> Option<(OutputField, Option<Enum>)> {
    let mut chars = line.chars();
    let mut field_name = String::new();
    let mut is_id = false;
    let mut is_read_only = false;
    loop {
        match chars.next() {
            None => break,
            Some(ch) if ch == ':' => break,
            Some('*') if field_name.is_empty() => is_id = true,
            Some('!') if field_name.is_empty() => is_read_only = true,
            Some(ch) if !ch.is_whitespace() => field_name.push(ch),
            Some(_) => {}
        }
    }
    let field_type = Mutex::new(RefCell::new(String::new()));
    let mut field_type_components: Vec<String> = vec![];
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
        match chars.next() {
            None => break,
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
                field_type: String::from("String"),
                id: is_id,
                read_only: is_read_only,
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
                id: is_id,
                read_only: is_read_only,
            },
            None,
        ))
    } else {
        Some((
            OutputField {
                field_name: String::from(trimmed_name),
                field_type: name2rust(trimmed_name, true),
                id: is_id,
                read_only: is_read_only,
            },
            Some(Enum {
                values: field_type_components,
            }),
        ))
    };
}
fn field_description_name(field_name: &str) -> String {
    format!(
        "{}_FIELD_DESCRIPTION",
        expand_field_name(&field_name).to_ascii_uppercase()
    )
}
