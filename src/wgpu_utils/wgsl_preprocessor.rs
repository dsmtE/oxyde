
use std::collections::hash_map;

use std::path::PathBuf;

use wgpu;

pub struct WGSLShaderBuilder {
	source: String,
	include_paths: Vec<PathBuf>,
}

pub struct ShaderModuleWrapper {
    pub module: wgpu::ShaderModule,
}

impl WGSLShaderBuilder {
    pub fn new(source: String) -> Self {
        Self {
            source,
            include_paths: Vec::new(),
        }
    }

    pub fn add_include_from_folder(self, path: &str) -> Self {
        self.add_include_path(PathBuf::from(path))
    }

    pub fn add_include_path(self, path: PathBuf) -> Self {
        self.add_include_paths(vec![path])
    }

    pub fn add_include_paths(mut self, paths: Vec<PathBuf>) -> Self {
        self.include_paths.extend(paths);
        self
    }

    pub fn build(self) -> Result<wgpu::ShaderSource<'static>, String> {
        let mut includes_replacement: hash_map::HashMap<&str, std::path::PathBuf> = hash_map::HashMap::new();
        let current_dir_path = std::env::current_dir().unwrap();
        for line in self.source.lines() {
            if line.starts_with("//!include") {
                let include_filename = line.split_whitespace().skip(1).next().unwrap();
                
                // search for the include file in the include paths
                let mut include_found = false;
                for mut include_path in self.include_paths.clone().into_iter() {
                    if !include_path.is_absolute() {
                        include_path = current_dir_path.join(include_path);
                    }
                    include_path = include_path.join(include_filename);
                    if include_path.is_file() {
                        include_found = true;
                        includes_replacement.insert(include_filename, include_path);
                        break;
                    }
                }
                if !include_found {
                    return Err(format!("Include file not found: {}", include_filename));
                }
            }
        }

        let mut shader_code = self.source.clone();
        for (include_filename, include_path) in includes_replacement {
            let include_code = std::fs::read_to_string(include_path).unwrap();
            shader_code = shader_code.replace(&format!("//!include {}", include_filename), &include_code);
        }

        Ok(wgpu::ShaderSource::Wgsl(shader_code.into()))
    }
}