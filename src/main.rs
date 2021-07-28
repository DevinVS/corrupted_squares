use std::fs::File;
use std::sync::Arc;

use vulkano::{Version, impl_vertex};
use vulkano::buffer::{BufferUsage, CpuAccessibleBuffer};
use vulkano::command_buffer::AutoCommandBufferBuilder;
use vulkano::command_buffer::CommandBufferExecFuture;
use vulkano::command_buffer::CommandBufferUsage;
use vulkano::command_buffer::DynamicState;
use vulkano::command_buffer::PrimaryAutoCommandBuffer;
use vulkano::command_buffer::SubpassContents;
use vulkano::descriptor::DescriptorSet;
use vulkano::descriptor::descriptor_set::PersistentDescriptorSet;
use vulkano::device::Device;
use vulkano::device::DeviceExtensions;
use vulkano::device::Queue;
use vulkano::format::Format;
use vulkano::image::ImageAccess;
use vulkano::image::ImageDimensions;
use vulkano::image::ImageUsage;
use vulkano::image::ImmutableImage;
use vulkano::image::MipmapsCount;
use vulkano::image::SwapchainImage;
use vulkano::image::view::ImageView;
use vulkano::instance::Instance;
use vulkano::instance::PhysicalDevice;
use vulkano::pipeline::GraphicsPipeline;
use vulkano::pipeline::GraphicsPipelineAbstract;
use vulkano::pipeline::viewport::Viewport;
use vulkano::render_pass::Framebuffer;
use vulkano::render_pass::FramebufferAbstract;
use vulkano::render_pass::RenderPass;
use vulkano::render_pass::Subpass;
use vulkano::sampler::BorderColor;
use vulkano::sampler::Filter;
use vulkano::sampler::Sampler;
use vulkano::sampler::SamplerAddressMode;
use vulkano::swapchain::AcquireError;
use vulkano::swapchain::Swapchain;
use vulkano::swapchain::SwapchainCreationError;
use vulkano::sync;
use vulkano::sync::GpuFuture;
use vulkano::swapchain;

use vulkano::sync::NowFuture;
use vulkano_win::VkSurfaceBuild;

use winit::event_loop::{ControlFlow, EventLoop};
use winit::platform::run_return::EventLoopExtRunReturn;
use winit::window::Fullscreen;
use winit::window::Window;
use winit::window::WindowBuilder;
use winit::event::Event;
use winit::event::WindowEvent;


#[derive(Default, Debug, Clone)]
struct Vertex {
    pos: [f32; 2],
    texture_pos: [f32; 2],
    texture_index: u32
}
impl_vertex!(Vertex, pos, texture_pos, texture_index);


// Helper struct to create rectangles
struct Rect {
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    tex_index: u32
}

impl Rect {
    fn new(x: f32, y: f32, width: f32, height: f32, tex_index: u32) -> Rect {
        Rect {x, y, width, height, tex_index}
    }

    fn vertices(&self) -> [Vertex; 4] {
        [
            Vertex {
                pos: [self.x, self.y],
                texture_pos: [0.0, 0.0],
                texture_index: self.tex_index
            },
            Vertex {
                pos: [self.x+self.width, self.y],
                texture_pos: [1.0, 0.0],
                texture_index: self.tex_index
            },
            Vertex {
                pos: [self.x+self.width, self.y+self.height],
                texture_pos: [1.0, 1.0],
                texture_index: self.tex_index
            },
            Vertex {
                pos: [self.x, self.y+self.height],
                texture_pos: [0.0, 1.0],
                texture_index: self.tex_index
            },
        ]
    }

    fn indices(&self, index: u16) -> [u16; 6] {
        let start = index * 4;
        [start, start+1, start+2, start+2, start+3, start]
    }
}

fn main() {
    // Required extensions for rendering to a window
    let required_extensions = vulkano_win::required_extensions();

    // Create vulkan instance with required extensions
    let instance = Instance::new(None, Version::V1_1, &required_extensions, None).unwrap();

    // Choose first gpu that is available
    let mut gpus = PhysicalDevice::enumerate(&instance);
    let physical = gpus.next().unwrap();

    // Debug Info
    println!(
        "Using device: {} (type: {:?})",
        physical.properties().device_name.as_ref().unwrap(),
        physical.properties().device_type.unwrap(),
    );

    // Create the event_loop and rendering surface
    let mut event_loop = EventLoop::new();
    let surface = WindowBuilder::new()
        .with_always_on_top(true)
        .with_decorations(false)
        .with_fullscreen(Some(Fullscreen::Borderless(None)))
        .with_resizable(false)
        .build_vk_surface(&event_loop, instance.clone())
        .unwrap();

    // Get queue families for gpu operations
    let queue_family = physical
        .queue_families()
        .find(|&q| {
            q.supports_graphics() && surface.is_supported(q).unwrap_or(false)
        })
        .unwrap();

    // Instantiate logical device for rendering
    let device_ext = DeviceExtensions {
        khr_swapchain: true,
        ..DeviceExtensions::none()
    };
    let (device, mut queues) = Device::new(
        physical,
        physical.supported_features(),
        &device_ext,
        [(queue_family, 0.5)].iter().cloned()
    ).unwrap();

    // The only queue we need right now is for rendering, may need transfer queue later
    let queue = queues.next().unwrap();

    // Load shaders
    let vs = vs::Shader::load(device.clone()).unwrap();
    let fs = fs::Shader::load(device.clone()).unwrap();

    // Create swapchain
    let (mut swapchain, images) = {
        let caps = surface.capabilities(physical).unwrap();
        let composite_alpha = caps.supported_composite_alpha.iter().next().unwrap();

        // Internal format for images
        let format = caps.supported_formats[0].0;
        let dimensions: [u32; 2] = surface.window().inner_size().into();

        Swapchain::start(device.clone(), surface.clone())
            .num_images(caps.min_image_count)
            .format(format)
            .dimensions(dimensions)
            .usage(ImageUsage::color_attachment())
            .sharing_mode(&queue)
            .composite_alpha(composite_alpha)
            .build()
            .unwrap()
    };

    // Describe how the render pass works
    let render_pass = Arc::new(
        vulkano::single_pass_renderpass!(
            device.clone(),
            attachments: {
                color: {
                    load: Clear,
                    store: Store,
                    format: swapchain.format(),
                    samples: 1,
                }
            },
            pass: {
                color: [color],
                depth_stencil: {}
            }
        ).unwrap()
    );

    // Create textures for the rectangles
    let red_texture = load_texture("src/red.png", queue.clone());
    let blue_texture = load_texture("src/green.png", queue.clone());
    let green_texture = load_texture("src/blue.png", queue.clone());

    let sampler = Sampler::new(
        device.clone(),
        Filter::Nearest,
        Filter::Nearest,
        vulkano::sampler::MipmapMode::Nearest,
        SamplerAddressMode::ClampToBorder(BorderColor::FloatOpaqueWhite),
        SamplerAddressMode::ClampToBorder(BorderColor::FloatOpaqueWhite),
        SamplerAddressMode::ClampToBorder(BorderColor::FloatOpaqueWhite),
        0.0,
        1.0,
        0.0,
        0.0
    ).unwrap();

    // Render pipeline for square
    let pipeline = Arc::new(
        GraphicsPipeline::start()
            .vertex_input_single_buffer::<Vertex>()
            .vertex_shader(vs.main_entry_point(), ())
            .triangle_list()
            .viewports_dynamic_scissors_irrelevant(1)
            .fragment_shader(fs.main_entry_point(), ())
            .render_pass(Subpass::from(render_pass.clone(), 0).unwrap())
            .build(device.clone())
            .unwrap()
    );

    let layout = pipeline.layout().descriptor_set_layout(0).unwrap();
    let texture_descriptor_set = Arc::new(
        PersistentDescriptorSet::start(layout.clone())
            .enter_array().unwrap()
            .add_sampled_image(red_texture, sampler.clone()).unwrap()
            .add_sampled_image(blue_texture, sampler.clone()).unwrap()
            .add_sampled_image(green_texture, sampler.clone()).unwrap()
            .leave_array().unwrap()
            .build().unwrap()
    ) as Arc<dyn DescriptorSet + Send + Sync>;

    let mut dynamic_state = DynamicState {
        line_width: None,
        viewports: None,
        scissors: None,
        compare_mask: None,
        write_mask: None,
        reference: None,
    };

    // We now create some buffers that will store the shape of our squares
    let red_rect = Rect::new(-0.8, -0.8, 0.2, 0.2, 0);
    let green_rect = Rect::new(-0.4, -0.8, 0.2, 0.2, 1);
    let blue_rect = Rect::new(0.0, -0.8, 0.2, 0.2, 2);

    let vertices = [red_rect.vertices(), green_rect.vertices(), blue_rect.vertices()].concat();
    let indices = [red_rect.indices(0), green_rect.indices(1), blue_rect.indices(2)].concat();

    let vertex_buffer = CpuAccessibleBuffer::from_iter(
        device.clone(),
        BufferUsage::vertex_buffer(),
        false,
        vertices.iter().cloned()
    ).unwrap();

    let index_buffer = CpuAccessibleBuffer::from_iter(
        device.clone(),
        BufferUsage::all(),
        false,
        indices.iter().cloned()
    ).unwrap();

    // Actual framebuffers to draw to
    let mut framebuffers = window_size_dependent_setup(&images, render_pass.clone(), &mut dynamic_state);

    // Rendering Loop
    let mut recreate_swapchain = false;
    let mut previous_frame_end = Some(sync::now(device.clone()).boxed());


    event_loop.run_return(move |ev, _, control_flow| {
        *control_flow = ControlFlow::Poll;

        match ev {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => *control_flow=ControlFlow::Exit,
            Event::WindowEvent {
                event: WindowEvent::Resized(_),
                ..
            } => {
                recreate_swapchain = true;
            }
            Event::RedrawEventsCleared => {
                previous_frame_end.as_mut().unwrap().cleanup_finished();

                if recreate_swapchain {
                    let dimensions = surface.window().inner_size().into();
                    let (new_swapchain, new_images) =
                        match swapchain.recreate().dimensions(dimensions).build() {
                            Ok(r) => r,
                            // This error tends to happen when the user is manually resizing the window.
                            // Simply restarting the loop is the easiest way to fix this issue.
                            Err(SwapchainCreationError::UnsupportedDimensions) => return,
                            Err(e) => panic!("Failed to recreate swapchain: {:?}", e),
                        };

                    swapchain = new_swapchain;
                    // Because framebuffers contains an Arc on the old swapchain, we need to
                    // recreate framebuffers as well.
                    framebuffers = window_size_dependent_setup(&new_images,render_pass.clone(), &mut dynamic_state);
                    recreate_swapchain = false;
                }

                let (image_num, suboptimal, acquire_future) =
                    match swapchain::acquire_next_image(swapchain.clone(), None) {
                        Ok(r) => r,
                        Err(AcquireError::OutOfDate) => {
                            recreate_swapchain = true;
                            return;
                        }
                        Err(e) => panic!("Failed to acquire next image: {:?}", e),
                    };
                
                if suboptimal {
                    recreate_swapchain = true;
                }

                let clear_values = vec![[1.0, 1.0, 1.0, 1.0].into()];

                let mut builder = AutoCommandBufferBuilder::primary(
                    device.clone(),
                    queue.family(),
                    CommandBufferUsage::OneTimeSubmit
                ).unwrap();

                builder
                    .begin_render_pass(
                        framebuffers[image_num].clone(),
                        SubpassContents::Inline,
                        clear_values
                    ).unwrap()
                    .draw_indexed(
                        pipeline.clone(),
                        &dynamic_state,
                        vertex_buffer.clone(),
                        index_buffer.clone(),
                        texture_descriptor_set.clone(),
                        (),
                        vec![]
                    ).unwrap()
                    .end_render_pass()
                    .unwrap();
                
                let command_buffer = builder.build().unwrap();

                let future = previous_frame_end
                    .take()
                    .unwrap()
                    .join(acquire_future)
                    .then_execute(queue.clone(), command_buffer)
                    .unwrap()
                    .then_swapchain_present(queue.clone(), swapchain.clone(), image_num)
                    .then_signal_fence_and_flush();

                match future {
                    Ok(future) => {
                        previous_frame_end = Some(future.boxed());
                    }
                    Err(sync::FlushError::OutOfDate) => {
                        recreate_swapchain = true;
                        previous_frame_end = Some(sync::now(device.clone()).boxed());
                    }
                    Err(e) => {
                        println!("Failed to flush future: {:?}", e);
                        previous_frame_end = Some(sync::now(device.clone()).boxed());
                    }
                }
            },
            _ => {}
        }
    });
}

fn load_texture(path: &str, queue: Arc<Queue>) -> Arc<ImageView<Arc<ImmutableImage>>> {
    // Initialize png decoder
    let decoder = png::Decoder::new(File::open(path).unwrap());
    let (info, mut reader) = decoder.read_info().unwrap();

    // Get image dimensions
    let dimensions = ImageDimensions::Dim2d {
        width: info.width,
        height: info.height,
        array_layers: 1
    };

    // Read image data into buffer on cpu
    let mut image_data = vec![0; reader.output_buffer_size()];
    reader.next_frame(&mut image_data).unwrap();

    // Copy to image in gpu memory
    let (image, future) = ImmutableImage::from_iter(
        image_data.iter().cloned(),
        dimensions,
        MipmapsCount::One,
        Format::R8G8B8A8Srgb,
        queue
    ).unwrap();

    ImageView::new(image).unwrap()
}

fn window_size_dependent_setup(
    images: &[Arc<SwapchainImage<Window>>],
    render_pass: Arc<RenderPass>,
    dynamic_state: &mut DynamicState,
) -> Vec<Arc<dyn FramebufferAbstract + Send + Sync>> {
    let dimensions = images[0].dimensions().width_height();

    let viewport = Viewport {
        origin: [0.0, 0.0],
        dimensions: [dimensions[0] as f32, dimensions[1] as f32],
        depth_range: 0.0..1.0
    };

    dynamic_state.viewports = Some(vec![viewport]);
    
    images.iter()
        .map(|image| {
            let view = ImageView::new(image.clone()).unwrap();
            Arc::new(
                Framebuffer::start(render_pass.clone())
                    .add(view)
                    .unwrap()
                    .build()
                    .unwrap()
            ) as Arc<dyn FramebufferAbstract + Send + Sync>
        }).collect::<Vec<_>>()
}

mod vs {
    vulkano_shaders::shader! {
        ty: "vertex",
        path: "src/rect.vs"
    }
}

mod fs {
    vulkano_shaders::shader! {
        ty: "fragment",
        path: "src/rect.fs"
    }
}
