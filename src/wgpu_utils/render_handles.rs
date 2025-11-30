use wgpu;

#[derive(Debug)]
pub enum RenderHandleError {
    NoCompatibleDevice(wgpu::RequestDeviceError),
    AdapterRequestError,
    SurfaceCreationError(wgpu::CreateSurfaceError),
    SurfaceTextureFormatRgbaBgraError,
    SurfaceSizeError(u32, u32),
}

impl std::fmt::Display for RenderHandleError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RenderHandleError::NoCompatibleDevice(request_device_error) => {
                write!(f, "No compatible device: {}", request_device_error)
            }
            RenderHandleError::AdapterRequestError => write!(f, "Adapter request error"),
            RenderHandleError::SurfaceCreationError(create_surface_error) => {
                write!(f, "Surface creation error: {}", create_surface_error)
            }
            RenderHandleError::SurfaceTextureFormatRgbaBgraError => {
                write!(f, "Surface should support Rgba8Unorm or Bgra8Unorm")
            }
            RenderHandleError::SurfaceSizeError(width, height) => {
                write!(f, "Surface size error: {}x{}. Width and height must be greater than 0", width, height)
            }
        }
    }
}

impl std::error::Error for RenderHandleError {}

pub struct RenderInstance {
    instance: wgpu::Instance,
    pub devices: Vec<DeviceHandle>,
}

pub struct DeviceHandle {
    adapter: wgpu::Adapter,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
}

pub struct SurfaceHandle<'s> {
    pub surface: wgpu::Surface<'s>,
    pub config: wgpu::SurfaceConfiguration,
    pub device_handle_id: usize,
}

impl RenderInstance {
    pub fn new(backends: Option<wgpu::Backends>, flags: Option<wgpu::InstanceFlags>) -> Self {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: backends.unwrap_or(wgpu::Backends::from_env().unwrap_or(wgpu::Backends::PRIMARY)),
            flags: flags.unwrap_or(wgpu::InstanceFlags::default()),
            ..Default::default()
        });
        Self {
            instance,
            devices: Vec::new(),
        }
    }

    // Return the index of a device that is compatible with the given surface
    // If no compatible device is found, create a new device and return its index
    pub async fn device(&mut self, compatible_surface: Option<&wgpu::Surface<'_>>, power_preference: Option<wgpu::PowerPreference>) -> Result<usize, RenderHandleError> {
        let compatible_device_index = match compatible_surface {
            Some(surface) => self
                .devices
                .iter()
                .enumerate()
                .find(|(_, device_handle)| device_handle.adapter.is_surface_supported(surface))
                .map(|(i, _)| i),
            None => (!self.devices.is_empty()).then_some(0),
        };
        
        return match compatible_device_index {
            Some(index) => Ok(index),
            None => self.new_device(compatible_surface, power_preference).await,
        }
    }

    // Create a new device handle and return its index
    async fn new_device(&mut self, compatible_surface: Option<&wgpu::Surface<'_>>, power_preference: Option<wgpu::PowerPreference>) -> Result<usize, RenderHandleError> {
        let adapter: wgpu::Adapter = wgpu::util::initialize_adapter_from_env(&self.instance, compatible_surface).unwrap_or(
            self.instance
                    .request_adapter(&wgpu::RequestAdapterOptions {
                        power_preference: power_preference.unwrap_or(wgpu::PowerPreference::from_env().unwrap_or_default()),
                        force_fallback_adapter: false,
                        compatible_surface,
                    })
                    .await
                    .map_err(|_| RenderHandleError::AdapterRequestError)?
        );

        let features = adapter.features();
        let limits = wgpu::Limits::default();
        #[allow(unused_mut)]
        let mut maybe_features = wgpu::Features::CLEAR_TEXTURE;
        #[cfg(feature = "wgpu-profiler")]
        {
            maybe_features |= wgpu_profiler::GpuProfiler::ALL_WGPU_TIMER_FEATURES;
        };
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: None,
                    required_features: features & maybe_features,
                    required_limits: limits,
                    ..Default::default()
                },
            )
            .await
            .map_err(|e| RenderHandleError::NoCompatibleDevice(e))?;
        self.devices.push(DeviceHandle {
            adapter,
            device,
            queue,
        });
        Ok(self.devices.len() - 1)
    }

        /// Creates a new surface for the specified window and dimensions.
        pub async fn create_render_surface<'w>(
            &mut self,
            window: impl Into<wgpu::SurfaceTarget<'w>>,
            width: u32,
            height: u32,
            present_mode: wgpu::PresentMode,
            power_preference: Option<wgpu::PowerPreference>,
        ) -> Result<SurfaceHandle<'w>, RenderHandleError> {
            if width == 0 || height == 0 {
                return Err(RenderHandleError::SurfaceSizeError(width, height));
            }
            let surface = self.instance.create_surface(window.into()).map_err(|e| RenderHandleError::SurfaceCreationError(e))?;

            let device_handle_id: usize = self.device(Some(&surface), power_preference).await?;
    
            let device_handle = &self.devices[device_handle_id];
            let capabilities = surface.get_capabilities(&device_handle.adapter);
            let format = capabilities
                .formats
                .into_iter()
                .find(|it| matches!(it, wgpu::TextureFormat::Rgba8Unorm | wgpu::TextureFormat::Bgra8Unorm))
                .ok_or(RenderHandleError::SurfaceTextureFormatRgbaBgraError)?;
            
            let config = wgpu::SurfaceConfiguration {
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                format,
                width,
                height,
                present_mode,
                desired_maximum_frame_latency: 2,
                alpha_mode: wgpu::CompositeAlphaMode::Auto,
                view_formats: vec![],
            };
            let mut surface_handle = SurfaceHandle {
                surface,
                config,
                device_handle_id,
            };

            surface_handle.configure(&device_handle.device);
            Ok(surface_handle)
        }

        pub fn device_from_surface_handle(&self, surface_handle: &SurfaceHandle) -> &DeviceHandle {
            &self.devices[surface_handle.device_handle_id]
        }
}

impl SurfaceHandle<'_> {
    pub fn resize(&mut self, device: &wgpu::Device, width: u32, height: u32) -> Result<(), RenderHandleError> {
        if width == 0 || height == 0 {
            return Err(RenderHandleError::SurfaceSizeError(width, height));
        }
        self.config.width = width;
        self.config.height = height;
        self.configure(device);
        Ok(())
    }

    pub fn configure(&mut self, device: &wgpu::Device) {
        self.surface.configure(device, &self.config);
    }

    pub fn set_present_mode(&mut self, device: &wgpu::Device, present_mode: wgpu::PresentMode) {
        self.config.present_mode = present_mode;
        self.configure(device);
    }

    pub fn format(&self) -> wgpu::TextureFormat {
        self.config.format
    }

    pub fn get_current_texture(&self) -> Result<wgpu::SurfaceTexture, wgpu::SurfaceError> {
        self.surface.get_current_texture()
    }
}

