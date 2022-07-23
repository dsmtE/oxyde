use std::{
    borrow::Cow::Borrowed,
    cell::RefCell,
    ffi::OsStr,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};

pub struct ShaderModuleWithSourceFiles {
    pub module: wgpu::ShaderModule,
    // main source file and all includes
    pub source_files: Vec<PathBuf>,
}

// compile glsl shadermodule using spirv
// TODO: try wgpuglsl feature instead
pub fn load_glsl_shader_module_from_path(device: &wgpu::Device, path: &Path, entry_point_name: &'static str) -> Result<ShaderModuleWithSourceFiles> {
    let source_files = RefCell::new(vec![path.canonicalize().unwrap()]);

    let glsl_code = std::fs::read_to_string(&path).with_context(|| format!("Failed to read shader file \"{:?}\"", path))?;

    let kind = match path.extension().and_then(OsStr::to_str) {
        Some("frag") => shaderc::ShaderKind::Fragment,
        Some("vert") => shaderc::ShaderKind::Vertex,
        Some("comp") => shaderc::ShaderKind::Compute,
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
        options.set_generate_debug_info();

        options.add_macro_definition(
            "FRAGMENT_SHADER",
            Some(if kind == shaderc::ShaderKind::Fragment {
                "1"
            } else {
                "0"
            }),
        );
        options.add_macro_definition("VERTEX_SHADER", Some(if kind == shaderc::ShaderKind::Vertex { "1" } else { "0" }));
        options.add_macro_definition("COMPUTE_SHADER", Some(if kind == shaderc::ShaderKind::Compute { "1" } else { "0" }));

        if cfg!(debug_assertions) {
            options.add_macro_definition("DEBUG", Some("1"));
        } else {
            options.add_macro_definition("NDEBUG", Some("1"));
        }

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
                    source_files.borrow_mut().push(path.canonicalize().unwrap());
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

    Ok(ShaderModuleWithSourceFiles {
        module: device.create_shader_module(&wgpu::ShaderModuleDescriptor {
            label: Some(path.file_name().unwrap().to_str().unwrap()),
            source: wgpu::ShaderSource::SpirV(Borrowed(&compilation_artifact.as_binary())),
        }),
        source_files: source_files.into_inner(),
    })
}
