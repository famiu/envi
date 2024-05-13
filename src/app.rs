use wgpu::Surface;
use winit::application::ApplicationHandler;
use winit::event::{ElementState, KeyEvent, WindowEvent};
use winit::event_loop::ActiveEventLoop;
use winit::keyboard::{KeyCode, PhysicalKey};
#[cfg(platform_linux)]
use winit::platform::startup_notify::{self, EventLoopExtStartupNotify, WindowAttributesExtStartupNotify};
use winit::window::{Icon, Window, WindowId};

use crate::prelude::*;

#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
pub enum UserEvent {}

/// State of a winit window.
struct WindowState<'a> {
    /// Render surface.
    surface: Surface<'a>,
    /// Window size.
    size: winit::dpi::PhysicalSize<u32>,
    /// Configuration of render surface.
    config: wgpu::SurfaceConfiguration,
    /// Graphics device handle.
    device: wgpu::Device,
    /// Device command queue handle.
    queue: wgpu::Queue,
    /// The actual winit Window.
    window: Arc<Window>,
}

impl<'a> WindowState<'a> {
    async fn new(_app: &Application<'a>, window: Window) -> Self {
        let window = Arc::new(window);
        let size = window.as_ref().inner_size();

        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            ..Default::default()
        });

        let surface = instance
            .create_surface(window.clone())
            .expect("Failed to create surface");

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                compatible_surface: Some(&surface),
                ..Default::default()
            })
            .await
            .expect("Failed to retrieve device adapter");

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::default(),
                    label: None,
                },
                None, // Trace path
            )
            .await
            .expect("Failed to retrieve device");

        let surface_capabilities = surface.get_capabilities(&adapter);

        // Shader code in this tutorial assumes an sRGB surface texture. Using a different
        // one will result in all the colors coming out darker. If you want to support non
        // sRGB surfaces, you'll need to account for that when drawing to the frame.
        let surface_format = surface_capabilities
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .expect("Surface does not support any sRGB texture format");

        let config = {
            let default_config = surface
                .get_default_config(&adapter, size.width, size.height)
                .expect("Failed to get default surface configuration");

            wgpu::SurfaceConfiguration {
                format: surface_format,
                ..default_config
            }
        };

        Self {
            window,
            surface,
            device,
            queue,
            config,
            size,
        }
    }
}

/// Represents an Application.
///
/// Can contain multiple windows. Run using `winit::event_loop::EventLoop::run_app()`.
///
/// # Examples
///
/// ```
/// let app = App::new()
/// let event_loop = winit::event_loop::new().unwrap()l
/// event_loop.run_app()
/// ```
pub struct Application<'a> {
    name: &'a str,
    /// Runtime used for async operations.
    rt: &'a tokio::runtime::Runtime,
    /// Map of windows indexed by their ID.
    windows: HashMap<WindowId, WindowState<'a>>,
    /// Icon used for application.
    icon: Option<Icon>,
}

impl<'a> Application<'a> {
    /// Create new App.
    ///
    /// # Arguments
    ///
    /// * `name` - App name.
    /// * `rt` - Handle for runtime used for async operations.
    ///
    /// # Examples
    ///
    /// ```
    /// let app = App::new("foo", "assets/foo_icon.png");
    /// ```
    pub fn new(name: &'a str, rt: &'a tokio::runtime::Runtime) -> Result<Self, Box<dyn Error>> {
        Ok(Self {
            name,
            rt,
            icon: None,
            windows: Default::default(),
        })
    }

    /// Set application icon. Also sets icon for every window in the application.
    pub fn with_icon(mut self, icon: &[u8]) -> Self {
        info!("Loading icon");

        let (icon_rgba, icon_width, icon_height) = {
            let image = image::load_from_memory(icon)
                .expect("Failed to load icon image")
                .into_rgba8();
            let (width, height) = image.dimensions();
            let rgba = image.into_raw();
            (rgba, width, height)
        };

        self.icon = Some(Icon::from_rgba(icon_rgba, icon_width, icon_height).expect("Failed to create icon"));

        for window_state in self.windows.values() {
            window_state.window.set_window_icon(self.icon.clone());
        }

        self
    }

    /// Create a new application window.
    pub async fn create_window(
        &mut self,
        event_loop: &ActiveEventLoop,
        title: &str,
    ) -> Result<WindowId, Box<dyn Error>> {
        let mut window_attributes = Window::default_attributes()
            .with_title(title)
            .with_transparent(true)
            .with_window_icon(self.icon.clone());

        #[cfg(platform_linux)]
        // Remove activation token from current process because child processes may not support it.
        if let Some(token) = event_loop.read_token_from_env() {
            startup_notify::reset_activation_token_env();
            info!("Using token {token:?} to activate a window");
            window_attributes = window_attributes.with_activation_token(token);
        }

        #[cfg(platform_macos)]
        if let Some(tab_id) = _tab_id {
            window_attributes = window_attributes.with_tabbing_identifier(&tab_id);
        }

        let window = event_loop.create_window(window_attributes)?;
        let window_state = WindowState::new(self, window).await;
        let window_id = window_state.window.id();

        info!("Created new window with id={window_id:?}");
        self.windows.insert(window_id, window_state);

        Ok(window_id)
    }

    /// Close window, and exit event loop when all windows are closed.
    pub fn close_window(&mut self, event_loop: &ActiveEventLoop, window_id: &WindowId) {
        info!("Closing window with id={window_id:?}");
        self.windows.remove(window_id);

        if self.windows.is_empty() {
            info!("Exiting event loop");
            event_loop.exit();
        }
    }
}

impl<'a> ApplicationHandler<UserEvent> for Application<'a> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        info!("Resuming application");
        let _ = self.rt.block_on(self.create_window(event_loop, self.name));
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, window_id: WindowId, event: WindowEvent) {
        info!("Window Event: {event:?}");

        match event {
            WindowEvent::CloseRequested
            | WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        state: ElementState::Pressed,
                        physical_key: PhysicalKey::Code(KeyCode::Escape),
                        ..
                    },
                ..
            } => self.close_window(event_loop, &window_id),
            _ => {}
        }
    }
}
