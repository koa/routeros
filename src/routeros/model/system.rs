use crate::routeros::model::{RosFieldAccessor, RosFieldValue, RouterOsResource};
use ros_macro::RouterOsApiFieldAccess;
use std::fmt::Debug;

#[derive(Debug, Default, RouterOsApiFieldAccess, Clone)]
pub struct SystemResource {
    architecture_name: RosFieldValue<String>,
    board_name: RosFieldValue<String>,
    cpu: RosFieldValue<String>,
    cpu_frequency: RosFieldValue<u64>,
    factory_software: RosFieldValue<String>,
    free_memory: RosFieldValue<u64>,
    total_hdd_space: RosFieldValue<u64>,
    uptime: RosFieldValue<String>,
    write_sect_since_reboot: RosFieldValue<u64>,
    bad_blocks: RosFieldValue<u64>,
    build_time: RosFieldValue<String>,
    cpu_count: RosFieldValue<u16>,
    cpu_load: RosFieldValue<u8>,
    free_hdd_space: RosFieldValue<u64>,
    platform: RosFieldValue<String>,
    total_memory: RosFieldValue<u64>,
    version: RosFieldValue<String>,
    write_sect_total: RosFieldValue<u64>,
}
macro_rules! ros_struct {(
     $StructName:ident,
     $path: literal,
     $($element: ident: $ty: ty,)*
    ) => {
      #[derive(Debug, Default, RouterOsApiFieldAccess)]
      struct $StructName {
        $($element: RosFieldValue<$ty>,)*
      }
      impl RouterOsResource for $StructName {
        fn resource_path() -> &'static str {
          $path
        }
      }
    }
}
/*
ros_struct!(
    SysRes,
    "system/resource",
    architecture_name: String,
    board_name: String,
    cpu: String,
    cpu_frequency: u64,
    factory_software: String,
    free_memory: u64,
    total_hdd_space: u64,
    uptime: String,
    write_sect_since_reboot: u64,
    bad_blocks: u64,
    build_time: String,
    cpu_count: u16,
    cpu_load: u8,
    free_hdd_space: u64,
    platform: String,
    total_memory: u64,
    version: String,
    write_sect_total: u64,
);
 */
//impl RouterOsSingleResource for SystemResource {}

// impl SystemResource {}

/*
impl RouterOsResource for SystemResource {
fn write_field<K, V>(&mut self, key: K, value: V) -> Result<(), RosError>
where
    K: AsRef<str>,
    V: AsRef<str>,
    V: ToString,
{
    return if let Some(field) = self
        .fields_mut()
        .find(|e| e.0.eq(key.as_ref()))
        .map(|e| e.1)
    {
        field.set(value.as_ref())?;
        Ok(())
    } else {
        Err(RosError::from(format!("Unknown key: {}", key.as_ref())))
    };
}

fn build(self) -> SystemResource {
    self
}

}
 */

/*
struct FieldDumper<'a, 'b> {
    f: &'b mut Formatter<'a>,
    needs_delimiter: bool,
}

impl<'a, 'b> FieldDumper<'a, 'b> {
    fn new<N: Display>(
        f: &'b mut Formatter<'a>,
        name: N,
    ) -> result::Result<FieldDumper<'a, 'b>, Error> {
        write!(f, "{}[", name)?;
        Ok(FieldDumper {
            f,
            needs_delimiter: false,
        })
    }
    fn field<V: Display>(
        &mut self,
        field_name: &str,
        field_content: &Option<V>,
    ) -> std::fmt::Result {
        if let Some(value) = field_content.as_ref() {
            if self.needs_delimiter {
                write!(self.f, ", ")?
            } else {
                self.needs_delimiter = true;
            }
            write!(self.f, "{}={}", field_name, value)
        } else {
            Ok(())
        }
    }
}

impl<'a, 'b> Drop for FieldDumper<'a, 'b> {
    fn drop(&mut self) {
        write!(self.f, "]").unwrap();
    }
}
*/
/*
impl Display for SystemResource {
    fn fmt<'a>(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let mut d = FieldDumper::new(f, "SystemResource")?;
        d.field("architecture-name", &self.architecture_name)?;
        d.field("board-name", &self.board_name)?;
        d.field("cpu", &self.cpu)?;
        d.field("cpu-frequency", &self.cpu_frequency)?;
        d.field("factory-software", &self.factory_software)?;
        Ok(())
    }
}
impl Display for SystemResource {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        RouterOsResource::fmt(self, f)
    }
}
*/
/*
impl RouterOsApiFieldAccess for SystemResource {
    fn fields_mut(&mut self) -> Box<dyn Iterator<Item = (&str, &mut dyn RosFieldAccessor)> + '_> {
        let fields: Vec<(&str, &mut dyn RosFieldAccessor)> = vec![
            ("uptime", &mut self.uptime),
            ("version", &mut self.version),
            ("build-time", &mut self.build_time),
            ("factory-software", &mut self.factory_software),
            ("free-memory", &mut self.free_memory),
            ("total-memory", &mut self.total_memory),
            ("cpu", &mut self.cpu),
            ("cpu-count", &mut self.cpu_count),
            ("cpu-frequency", &mut self.cpu_frequency),
            ("cpu-load", &mut self.cpu_load),
            ("free-hdd-space", &mut self.free_hdd_space),
            ("total-hdd-space", &mut self.total_hdd_space),
            ("write-sect-since-reboot", &mut self.write_sect_since_reboot),
            ("write-sect-total", &mut self.write_sect_total),
            ("bad-blocks", &mut self.bad_blocks),
            ("architecture-name", &mut self.architecture_name),
            ("board-name", &mut self.board_name),
            ("platform", &mut self.platform),
        ];
        Box::new(fields.into_iter())
    }

    fn fields(&self) -> Box<dyn Iterator<Item = (&str, &dyn RosFieldAccessor)> + '_> {
        let fields: Vec<(&str, &dyn RosFieldAccessor)> = vec![
            ("uptime", &self.uptime),
            ("version", &self.version),
            ("build-time", &self.build_time),
            ("factory-software", &self.factory_software),
            ("free-memory", &self.free_memory),
            ("total-memory", &self.total_memory),
            ("cpu", &self.cpu),
            ("cpu-count", &self.cpu_count),
            ("cpu-frequency", &self.cpu_frequency),
            ("cpu-load", &self.cpu_load),
            ("free-hdd-space", &self.free_hdd_space),
            ("total-hdd-space", &self.total_hdd_space),
            ("write-sect-since-reboot", &self.write_sect_since_reboot),
            ("write-sect-total", &self.write_sect_total),
            ("bad-blocks", &self.bad_blocks),
            ("architecture-name", &self.architecture_name),
            ("board-name", &self.board_name),
            ("platform", &self.platform),
        ];
        Box::new(fields.into_iter())
    }
}

 */
impl RouterOsResource for SystemResource {
    fn resource_path() -> &'static str {
        "system/resource"
    }
}
