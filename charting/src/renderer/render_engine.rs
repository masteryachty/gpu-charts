use std::{cell::RefCell, rc::Rc};

use super::data_store::DataStore;
use crate::{calcables::min_max::calculate_min_max_y, drawables::plot::RenderListener};
use futures::channel::oneshot;
use getrandom::Error;
// use web_sys;
use winit::window::Window;

#[cfg(target_arch = "wasm32")]
use winit::platform::web::WindowExtWebSys;

pub struct RenderEngine {
    // instance: wgpu::Instance,
    surface: wgpu::Surface<'static>,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub config: wgpu::SurfaceConfiguration,
    // window: std::rc::Rc<Window>,
    render_listeners: Vec<Box<dyn RenderListener>>,
    data_store: Rc<RefCell<DataStore>>,
}

impl RenderEngine {
    pub async fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        // Create a command encoder for the compute work.
        let mut command_encoder =
            self.device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Compute Encoder"),
                });

        log::info!(
            "1 {:?} {:?}",
            self.data_store.borrow().start_x,
            self.data_store.borrow().end_x
        );
        // Calculate min/max values and get the two staging buffers.
        let (min_max_buffer, staging_buffer) = calculate_min_max_y(
            &self.device,
            &self.queue,
            &mut command_encoder,
            &self.data_store.borrow(),
            self.data_store.borrow().start_x,
            self.data_store.borrow().end_x,
        );
        log::info!("2");

        // Submit GPU commands.
        self.queue.submit(std::iter::once(command_encoder.finish()));
        log::info!("3");

        // Force the GPU to finish its work.
        self.device.poll(wgpu::Maintain::Wait);
        self.device.poll(wgpu::Maintain::Wait);
        log::info!("4");

        // Prepare to asynchronously map the staging buffer.
        let buffer_slice = staging_buffer.slice(..);
        let (sender, receiver) = oneshot::channel();

        buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
            log::info!("Mapping callback triggered");
            // Send whether mapping was successful.
            let _ = sender.send(result.is_ok());
        });

        // let start_time = std::time::Instant::now();
        // let timeout = std::time::Duration::from_secs(5);

        let mapping_succeeded = receiver.await.unwrap_or(false);
        if !mapping_succeeded {
            log::error!("Failed to map staging buffer for reading");
            return Err(wgpu::SurfaceError::Lost);
        }

        // Mapping succeeded
        log::info!("7");

        // Read values from the mapped buffer.
        let (miny, maxy) = {
            let data = buffer_slice.get_mapped_range();
            let values: &[f32] = bytemuck::cast_slice(&data);
            log::info!("Mapped values: {:?}", values);
            let miny = values[0];
            let maxy = values[1];
            log::info!("Extracted miny: {} maxy: {}", miny, maxy);
            self.data_store.borrow_mut().update_min_max_y(miny, maxy);
            (miny, maxy)
        };

        // Unmap the buffer now that we have read its data.
        staging_buffer.unmap();

        // Update your data store with the new min and max values.
        self.data_store
            .borrow_mut()
            .update_buffers(&self.device, min_max_buffer);

        // Create a new command encoder for the render pass.
        let mut render_encoder =
            self.device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Render Encoder"),
                });

        {
            let drawable = self.surface.get_current_texture()?;
            let image_view = drawable
                .texture
                .create_view(&wgpu::TextureViewDescriptor::default());
            self.notify_render_listeners(&mut render_encoder, &image_view);
            self.queue.submit(Some(render_encoder.finish()));
            let _keep_alive = drawable;

            _keep_alive.present();
        }

        log::info!("Rendered with min_y: {}, max_y: {}", miny, maxy);
        Ok(())
        //         loop {
        //             self.device.poll(wgpu::Maintain::Wait);
        //             if let Ok(Some(true)) = receiver.try_recv() {
        //     break;
        // }
        //             // let mapped = receiver.try_recv() {
        //             // log::info!("Mapping callback triggered {:?}", mapped);

        //             //     // if let Some(mapped) = mapped {
        //             //     //     break;
        //             //     // } else {
        //             //         log::error!("Failed to map staging buffer for reading");
        //             //         return Err(wgpu::SurfaceError::Lost);
        //             //     // }
        //             // }

        //             if start_time.elapsed() > timeout {
        //                 log::error!("Buffer mapping timed out after 5 seconds");
        //                 return Err(wgpu::SurfaceError::Lost);
        //             }
        // }
    }

    pub async fn new(
        window: std::rc::Rc<Window>,
        data_store: Rc<RefCell<DataStore>>,
    ) -> Result<Self, Error> {
        let mut t = wgpu::InstanceDescriptor {
            backends: wgpu::Backends::BROWSER_WEBGPU,
            flags: wgpu::InstanceFlags::default(),
            ..Default::default()
        };
        t.flags.insert(wgpu::InstanceFlags::VALIDATION);
        t.flags.insert(wgpu::InstanceFlags::DEBUG);
        // log::info!("a");

        let instance = wgpu::Instance::new(&t);
        let surface = {
            use wgpu::SurfaceTarget;
            instance
                .create_surface(SurfaceTarget::Canvas(
                    window.canvas().expect("Window should have a canvas"),
                ))
                .unwrap()
        };
        // get time in milliseconds
        // let performance = web_sys::window().unwrap().performance().unwrap();
        // let start = performance.now();

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                compatible_surface: Some(&surface),
                power_preference: wgpu::PowerPreference::HighPerformance,
                ..Default::default()
            })
            .await
            .unwrap();

        // let limits = adapter.limits();

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::default(),
                    label: Some("Device"),
                    ..Default::default()
                },
                None,
            )
            .await
            .unwrap();
        // log::info!("c");

        // let end = performance.now();
        // log::info!("Time taken: {} ms", end - start);

        // let Some(surface_config) =
        //     surface.get_default_config(&adapter, width.max(1), height.max(1))
        // else {
        //     return Err(GraphicsError::IncompatibleAdapter);
        // };

        let surface_capabilities = surface.get_capabilities(&adapter);
        let surface_format = surface_capabilities.formats[0];
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: data_store.borrow().screen_size.width,
            height: data_store.borrow().screen_size.height,
            present_mode: surface_capabilities.present_modes[0],
            alpha_mode: surface_capabilities.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);

        // data_store
        //     .borrow_mut()
        //     .set_x_range(1739785500, 1739811799, &device);

        Ok(Self {
            // window: window.clone(),
            data_store,
            // instance,
            surface,
            device,
            queue,
            config,
            render_listeners: Vec::new(),
        })
    }

    pub fn resized(&mut self, width: u32, height: u32) {
        self.config.width = width;
        self.config.height = height;
        self.surface.configure(&self.device, &self.config);
        log::info!("Resized surface to {{ width: {width}, height: {height} }}");
    }

    // Add a listener
    pub fn add_render_listener(&mut self, listener: Box<dyn RenderListener>) {
        self.render_listeners.push(listener);
    }

    // Notify all listeners
    pub fn notify_render_listeners(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        image_view: &wgpu::TextureView,
    ) {
        for listener in &mut self.render_listeners {
            listener.on_render(
                &self.queue,
                &self.device,
                encoder,
                image_view,
                self.data_store.clone(),
            );
        }
    }
}

// #[derive(Debug)]
// pub enum GraphicsError {
//     // NoCompatibleAdapter,
//     // IncompatibleAdapter,
//     RequestDeviceError(Box<wgpu::RequestDeviceError>),
//     CreateSurfaceError(Box<wgpu::CreateSurfaceError>),
// }
