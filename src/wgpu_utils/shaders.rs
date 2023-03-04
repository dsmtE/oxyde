use std::{
    cell::RefCell,
    ffi::OsStr,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};

pub enum Source {
    File(PathBuf),
    Code(String),
}

pub use shaderc::ShaderKind;

pub struct ShaderModuleWithSourceFiles {
    pub module: wgpu::ShaderModule,
    // main source file and all includes
    pub source_files: Vec<Source>,
}

// compile glsl shadermodule using spirv
// TODO: try wgpuglsl feature instead
pub fn load_glsl_shader_module_from_path(device: &wgpu::Device, path: &Path, entry_point_name: &'static str) -> Result<ShaderModuleWithSourceFiles> {
    let source_files = RefCell::new(vec![Source::File(path.canonicalize().unwrap())]);

    let glsl_code = std::fs::read_to_string(&path).with_context(|| format!("Failed to read shader file \"{:?}\"", path))?;

    let kind = match path.extension().and_then(OsStr::to_str) {
        Some("frag") => ShaderKind::Fragment,
        Some("vert") => ShaderKind::Vertex,
        Some("comp") => ShaderKind::Compute,
        _ => {
            return Err(anyhow::anyhow!("Did not recognize file extension for shader file \"{:?}\"", path));
        },
    };

    let compilation_artifact = {
        let compiler = shaderc::Compiler::new().unwrap();
        let mut options = shaderc::CompileOptions::new().unwrap();

        options.set_warnings_as_errors();
        options.set_target_env(shaderc::TargetEnv::Vulkan, 0);
        options.set_optimization_level(shaderc::OptimizationLevel::Performance);

        options.add_macro_definition("FRAGMENT_SHADER", Some(if kind == ShaderKind::Fragment { "1" } else { "0" }));
        options.add_macro_definition("VERTEX_SHADER", Some(if kind == ShaderKind::Vertex { "1" } else { "0" }));
        options.add_macro_definition("COMPUTE_SHADER", Some(if kind == ShaderKind::Compute { "1" } else { "0" }));
        options.add_macro_definition(if cfg!(debug_assertions) {"DEBUG"} else {"NDEBUG"} , Some("1"));

        options.set_include_callback(|name, include_type, source_file, _depth| {
            let path = if include_type == shaderc::IncludeType::Relative {
                Path::new(Path::new(source_file).parent().unwrap()).join(name)
            } else {
                return Err(format!(
                    "Unable to handle standard IncludeType for shader file\"{:?}\" for include \"{:?}\"",
                    source_file, name
                ));
            };
            match std::fs::read_to_string(&path) {
                Ok(glsl_code) => {
                    source_files.borrow_mut().push(Source::File(path.canonicalize().unwrap()));
                    Ok(shaderc::ResolvedInclude {
                        resolved_name: String::from(name),
                        content: glsl_code,
                    })
                },
                Err(err) => Err(format!(
                    "Failed to resolve include to {} in {} (was looking for {:?}): {}",
                    name, source_file, path, err
                )),
            }
        });

        compiler
            .compile_into_spirv(&glsl_code, kind, path.to_str().unwrap(), entry_point_name, Some(&options))
            .with_context(|| format!("Failed to compile shader {:?}", path))?
    };

    if compilation_artifact.get_num_warnings() > 0 {
        warn!("warnings when compiling {:?}:\n{}", path, compilation_artifact.get_warning_messages());
    }

    let label = Some(path.file_name().unwrap().to_str().unwrap());

    let module =  device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label,
        source: wgpu::util::make_spirv(compilation_artifact.as_binary_u8()),
    });

    Ok(ShaderModuleWithSourceFiles {
        module,
        source_files: source_files.into_inner(),
    })
}

pub fn load_glsl_shader_module_from_string(
    device: &wgpu::Device,
    glsl_code: &String,
    kind: ShaderKind,
    entry_point_name: &'static str,
    include_paths: Vec<&'static str>,
    label: Option<&str>,

) -> Result<ShaderModuleWithSourceFiles> {

    let source_files = RefCell::new(vec![Source::Code(glsl_code.to_owned())]);

    let compilation_artifact = {
        let compiler = shaderc::Compiler::new().unwrap();
        let mut options = shaderc::CompileOptions::new().unwrap();

        options.set_warnings_as_errors();
        options.set_target_env(shaderc::TargetEnv::Vulkan, 0);
        options.set_optimization_level(shaderc::OptimizationLevel::Performance);

        options.add_macro_definition("FRAGMENT_SHADER", Some(if kind == ShaderKind::Fragment { "1" } else { "0" }));
        options.add_macro_definition("VERTEX_SHADER", Some(if kind == ShaderKind::Vertex { "1" } else { "0" }));
        options.add_macro_definition("COMPUTE_SHADER", Some(if kind == ShaderKind::Compute { "1" } else { "0" }));
        options.add_macro_definition(if cfg!(debug_assertions) {"DEBUG"} else {"NDEBUG"} , Some("1"));

        options.set_include_callback(|name, include_type, source_file, _depth| {
            if include_type == shaderc::IncludeType::Standard {
                return Err(format!(
                    "Unable to handle standard IncludeType for shader file\"{:?}\" for include \"{:?}\"",
                    source_file, name
                ));
            };
            
            let possible_paths = include_paths.iter()
                .map(|path| Path::new(&path).join(name))
                .filter(|path| path.exists()).collect::<Vec::<PathBuf>>();
            
            if possible_paths.len() == 0 {
                return Err(format!("Unable to find the file \"{}\" in listed include_paths",
                            name
                        ));
            }else if  possible_paths.len() > 1 {
                return Err(format!(
                    "Multiples files found for the same include name \"{}\" in listed include_paths",
                    name
                ));
            }

            let path = possible_paths.first().unwrap();
            match std::fs::read_to_string(path) {
                Ok(glsl_code) => {
                    debug!("Include to {} in {} resolved at path: {:?}", name, source_file, path);
                    source_files.borrow_mut().push(Source::File(path.canonicalize().unwrap()));
                    Ok(shaderc::ResolvedInclude {
                        resolved_name: String::from(name),
                        content: glsl_code,
                    })
                },
                Err(err) => Err(format!(
                    "Failed to resolve include to {} in {} (was looking for {:?}): {}",
                    name, source_file, path, err
                )),
            }
        });

        compiler
            .compile_into_spirv(&glsl_code, kind, label.unwrap_or("unknown"), entry_point_name, Some(&options))
            .with_context(|| "Failed to compile shader from string")?
    };
    
    if compilation_artifact.get_num_warnings() > 0 {
        warn!("warnings when compiling:\n{}", compilation_artifact.get_warning_messages());
    }

    let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label,
        source: wgpu::util::make_spirv(compilation_artifact.as_binary_u8()),
    });

    Ok(ShaderModuleWithSourceFiles {
        module,
        source_files: source_files.into_inner(),
    })
}