
use std::collections::HashMap;

use std::path::PathBuf;
use naga_oil::compose::{self, ComposableModuleDescriptor, Composer, ComposerError, NagaModuleDescriptor};

use anyhow::Result;

// TODO: use macro to generate this enum and conversion
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum ShaderDefValue {
    Bool(bool),
    Int(i32),
    UInt(u32),
}

impl From<bool> for ShaderDefValue { fn from(b: bool) -> ShaderDefValue { ShaderDefValue::Bool(b) } }
impl From<i32> for ShaderDefValue { fn from(i: i32) -> ShaderDefValue { ShaderDefValue::Int(i) } }
impl From<u32> for ShaderDefValue { fn from(u: u32) -> ShaderDefValue { ShaderDefValue::UInt(u) } }

impl From<ShaderDefValue> for compose::ShaderDefValue {
    fn from(value: ShaderDefValue) -> compose::ShaderDefValue {
        match value {
            ShaderDefValue::Bool(b) => compose::ShaderDefValue::Bool(b),
            ShaderDefValue::Int(i) => compose::ShaderDefValue::Int(i),
            ShaderDefValue::UInt(u) => compose::ShaderDefValue::UInt(u),
        }
    }
}

pub struct ShaderComposer {
    name: Option<&'static str>,
	source: &'static str,
    composer: Composer,
    defines: HashMap<String, compose::ShaderDefValue>,
}

impl ShaderComposer {
    pub fn new(source: &'static str, name: Option<&'static str>) -> Self {
        Self {
            name,
            source,
            composer: Composer::default(),
            defines: HashMap::new(),
        }
    }

    pub fn add_module_read_from_path(&mut self, mut path: std::borrow::Cow<PathBuf>) ->  Result<()> {
        if !path.is_absolute() {
            *path.to_mut() = std::env::current_dir()?.join(path.as_ref());
        }
        
        let source = std::fs::read_to_string(path.as_ref())?;
        let name = path.file_name().unwrap().to_str().unwrap();

        self.add_module(name, source.as_str())?;

        Ok(())
    }

    pub fn add_module<'a>(&mut self, name: &'a str, source: &'a str) -> Result<(), ComposerError> {
        self.composer.add_composable_module(ComposableModuleDescriptor {
            source,
            file_path: name,
            ..Default::default()
        })
        .map(|_| ())
    }

    pub fn with_shader_define(mut self, name: &str, value: ShaderDefValue) -> Self {
        self.add_shader_define(name, value);
        self
    }

    pub fn defines(&self) -> &HashMap<String, compose::ShaderDefValue> {
        &self.defines
    }

    pub fn add_shader_define(&mut self, name: &str, value: ShaderDefValue) {
        self.defines.insert(name.to_string(), value.into());
    }

    pub fn build_ref(&mut self) -> Result<wgpu::naga::Module, ComposerError> {
        self.composer
            .make_naga_module(NagaModuleDescriptor {
                source: self.source,
                file_path: self.name.unwrap_or("unknown"),
                shader_defs: self.defines.clone(),
                ..Default::default()
            })
    }

    pub fn build(mut self) -> Result<wgpu::naga::Module, ComposerError> {
        self.composer
            .make_naga_module(NagaModuleDescriptor {
                source: self.source,
                file_path: self.name.unwrap_or("unknown"),
                shader_defs: self.defines,
                ..Default::default()
            })
    }
}