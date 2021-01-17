use vulkano_text::{DrawText, DrawTextTrait};

use vulkano::command_buffer::{AutoCommandBufferBuilder, DynamicState, SubpassContents};
use vulkano::device::{Device, DeviceExtensions};
use vulkano::format::Format;
use vulkano::framebuffer::{Framebuffer, FramebufferAbstract, RenderPassAbstract};
use vulkano::image::attachment::AttachmentImage;
use vulkano::image::SwapchainImage;
use vulkano::instance::{Instance, PhysicalDevice};
use vulkano::pipeline::viewport::Viewport;
use vulkano::swapchain::{AcquireError, PresentMode, SurfaceTransform, Swapchain, SwapchainCreationError, ColorSpace, FullscreenExclusive};
use vulkano::swapchain;
use vulkano::sync::{GpuFuture, FlushError};
use vulkano::sync;

use vulkano_win::VkSurfaceBuild;

use winit::window::{WindowBuilder, Window};
use winit::event_loop::{EventLoop, ControlFlow};
use winit::event::{Event, WindowEvent};

use std::sync::Arc;
use std::time::Instant;
use std::env;

fn window_size_dependent_setup(
    device: Arc<Device>,
    images: &[Arc<SwapchainImage<Window>>],
    render_pass: Arc<dyn RenderPassAbstract + Send + Sync>,
    dynamic_state: &mut DynamicState
) -> Vec<Arc<dyn FramebufferAbstract + Send + Sync>> {
    let dimensions = images[0].dimensions();

    let viewport = Viewport {
        origin: [0.0, 0.0],
        dimensions: [dimensions[0] as f32, dimensions[1] as f32],
        depth_range: 0.0 .. 1.0,
    };
    dynamic_state.viewports = Some(vec!(viewport));

    let depthbuffer = AttachmentImage::transient(device.clone(), images[0].dimensions(), Format::D16Unorm).unwrap();
    images.iter().map(|image| {
        Arc::new(
            Framebuffer::start(render_pass.clone())
                .add(image.clone()).unwrap()
                .add(depthbuffer.clone()).unwrap()
                .build().unwrap()
        ) as Arc<dyn FramebufferAbstract + Send + Sync>
    }).collect::<Vec<_>>()
}

fn main() {
    let lines = vec!(
        "Lorem ipsum dolor sit amet, consectetur adipiscing elit.",
        "Quisque nec lorem auctor, lobortis nulla congue, ultrices justo.",
        "Vivamus ultrices, elit quis porttitor dapibus, nisi odio fringilla arcu, vitae finibus odio lorem vel mi.",
        "Maecenas laoreet in metus et mollis.",
        "Nullam et velit dui.",
        "Quisque gravida a tortor eu pulvinar.",
        "Maecenas vitae quam nibh.",
        "Aenean lacus urna, pulvinar non vulputate vel, sollicitudin nec mauris.",
        "Integer lobortis lorem at gravida varius.",
        "Aliquam tristique, massa sed aliquet sagittis, risus erat fermentum quam, sit amet rhoncus lectus velit sit amet massa.",
        "Aenean sit amet augue urna.",
        "In porttitor dignissim erat, aliquet lacinia sapien molestie eu.",
        "Pellentesque ut pellentesque odio, id efficitur dui.",
        "Morbi ligula diam, consequat sed neque sed, posuere blandit libero.",
        "Etiam interdum pellentesque justo et vehicula.",
        "Mauris sagittis quis ante egestas luctus.",
        "",
        "Aliquam volutpat consequat nisl at tincidunt.",
        "Nam congue tellus ut est gravida interdum.",
        "Integer ut hendrerit purus.",
        "Vestibulum lobortis magna et finibus iaculis.",
        "Nam faucibus tortor id nibh placerat iaculis.",
        "Donec arcu arcu, eleifend sit amet ultrices a, consequat in ante.",
        "Sed accumsan velit dui, ac tempus lorem tempor at.",
        "Donec facilisis urna eu scelerisque volutpat.",
        "Nunc sed leo nulla.",
        "Mauris orci leo, ultricies a diam id, iaculis dapibus nibh.",
        "Nunc auctor purus vel lobortis viverra.",
        "Curabitur vitae mattis nulla, vitae vulputate leo.",
        "Mauris lacinia ultricies ullamcorper.",
        "Nullam ultrices augue nec commodo tristique.",
        "Ut et tellus sagittis, sodales elit et, vestibulum arcu.",
        "Cras dui arcu, consectetur in urna vel, lobortis elementum augue.",
        "",
        "Donec consequat orci ac commodo ultricies.",
        "Pellentesque mattis felis ut enim consequat feugiat.",
        "Vestibulum et congue sapien.",
        "Cras sem urna, condimentum sed hendrerit vitae, accumsan et orci.",
        "Etiam vitae finibus odio.",
        "Cras finibus sem sed ante varius, non posuere lectus sollicitudin.",
        "Nunc vestibulum odio at elit pharetra finibus.",
        "Pellentesque habitant morbi tristique senectus et netus et malesuada fames ac turpis egestas.",
        "Morbi varius pulvinar mauris et porttitor.",
        "Duis tincidunt vel nisl in convallis.",
        "Proin scelerisque libero nec eros aliquam lacinia.",
        "Phasellus mauris sem, ultrices non pharetra rutrum, molestie vitae dui.",
        "Fusce vulputate quam in maximus consectetur.",
        "Nulla at luctus ex.",
        "Curabitur pretium augue erat, in cursus dui hendrerit ut.",
        "",
        "Nulla viverra semper ligula porta consectetur.",
        "Lorem ipsum dolor sit amet, consectetur adipiscing elit.",
        "Etiam sit amet luctus erat, ac ultrices felis.",
        "Nunc placerat molestie luctus.",
        "Cras hendrerit lectus eget venenatis sodales.",
        "Vivamus hendrerit nulla vel magna mattis, a vehicula mauris elementum.",
        "Nunc euismod ut nisi pulvinar vulputate.",
        "Nullam ut leo eget mi aliquam interdum.",
        "Pellentesque sed nunc ac metus consectetur aliquam.",
        "Proin gravida tincidunt ex, et interdum ex tristique a.",
        "Maecenas fringilla gravida eros, eu interdum risus mattis consectetur.",
        "",
        "Fusce in malesuada risus, ultrices sollicitudin justo.",
        "Suspendisse dolor purus, tincidunt ac ultrices ac, blandit nec massa.",
        "Duis a consequat metus.",
        "Vestibulum condimentum ultrices varius.",
        "Sed nec convallis nibh.",
        "Vestibulum ante ipsum primis in faucibus orci luctus et ultrices posuere cubilia Curae; Nulla hendrerit cursus orci eu venenatis.",
        "Aenean condimentum enim vel metus pulvinar, sed elementum nulla sodales.",
        "Vivamus volutpat fermentum mauris vel mattis.",
    );
    let mut args = env::args();
    args.next();
    let benchmark_count = match args.next() {
        Some(arg) => arg.parse().ok(),
        None      => None,
    };

    let required_extensions = vulkano_win::required_extensions();
    let instance = Instance::new(None, &required_extensions, None).unwrap();
    let physical = PhysicalDevice::enumerate(&instance).next().unwrap();
    println!("Using device: {} (type: {:?})", physical.name(), physical.ty());

    let event_loop = EventLoop::new();
    let surface = WindowBuilder::new().build_vk_surface(&event_loop, instance.clone()).unwrap();
    let queue_family = physical.queue_families().find(|&q| {
        q.supports_graphics() && surface.is_supported(q).unwrap_or(false)
    }).unwrap();
    let device_ext = DeviceExtensions { khr_swapchain: true, .. DeviceExtensions::none() };
    let (device, mut queues) = Device::new(physical, physical.supported_features(), &device_ext,
        [(queue_family, 0.5)].iter().cloned()).unwrap();
    let queue = queues.next().unwrap();
    let (mut swapchain, images) = {
        let caps = surface.capabilities(physical).unwrap();
        let usage = caps.supported_usage_flags;
        let alpha = caps.supported_composite_alpha.iter().next().unwrap();
        let format = caps.supported_formats[0].0;
        let dimensions: [u32; 2] = surface.window().inner_size().into();
        Swapchain::new(device.clone(), surface.clone(), caps.min_image_count, format,
            dimensions, 1, usage, &queue, SurfaceTransform::Identity, alpha,
            PresentMode::Fifo, FullscreenExclusive::Default, true, ColorSpace::SrgbNonLinear).unwrap()

    };

    // include a depth buffer (unlike triangle.rs) to ensure vulkano-text isnt dependent on a specific render_pass
    let render_pass = Arc::new(vulkano::single_pass_renderpass!(
        device.clone(),
        attachments: {
            color: {
                load: Clear,
                store: Store,
                format: swapchain.format(),
                samples: 1,
            },
            depth: {
                load: Clear,
                store: DontCare,
                format: Format::D16Unorm,
                samples: 1,
            }
        },
        pass: {
            color: [color],
            depth_stencil: {depth}
        }
    ).unwrap());

    let mut draw_text = DrawText::new(device.clone(), queue.clone(), swapchain.clone(), &images);

    let (width, _): (u32, u32) = surface.window().inner_size().into();
    let mut x = 0.0;

    let mut dynamic_state = DynamicState { line_width: None, viewports: None, scissors: None, compare_mask: None, write_mask: None, reference: None };
    let mut framebuffers = window_size_dependent_setup(device.clone(), &images, render_pass.clone(), &mut dynamic_state);
    let mut recreate_swapchain = false;
    let mut previous_frame_end = Some(Box::new(sync::now(device.clone())) as Box<dyn GpuFuture>);

    let start = Instant::now();
    let mut frames_rendered = 0;
    event_loop.run(move |event, _, control_flow| {
        match event {
            Event::WindowEvent { event: WindowEvent::CloseRequested, .. } => {
                *control_flow = ControlFlow::Exit;
            },
            Event::WindowEvent { event: WindowEvent::Resized(_), .. } => {
                recreate_swapchain = true;
            },
            Event::RedrawEventsCleared => {
                previous_frame_end.as_mut().unwrap().cleanup_finished();
                if recreate_swapchain {
                    let dimensions: [u32; 2] = surface.window().inner_size().into();
                    let (new_swapchain, new_images) = match swapchain.recreate_with_dimensions(dimensions) {
                        Ok(r) => r,
                        Err(SwapchainCreationError::UnsupportedDimensions) => return,
                        Err(e) => panic!("Failed to recreate swapchain: {:?}", e)
                    };

                    swapchain = new_swapchain;
                    framebuffers = window_size_dependent_setup(device.clone(), &new_images, render_pass.clone(), &mut dynamic_state);

                    draw_text = DrawText::new(device.clone(), queue.clone(), swapchain.clone(), &new_images);

                    recreate_swapchain = false;
                }

                if x > width as f32 {
                    x = 0.0;
                }
                else {
                    x += 2.0;
                }
                
                for (i, line) in lines.iter().enumerate() {
                    draw_text.queue_text(x, (i + 1) as f32 * 15.0, 15.0, [1.0, 1.0, 1.0, 1.0], line);
                }
                
                let (image_num, suboptimal, acquire_future) = match swapchain::acquire_next_image(swapchain.clone(), None) {
                    Ok(r) => r,
                    Err(AcquireError::OutOfDate) => {
                        recreate_swapchain = true;
                        return;
                    },
                    Err(e) => panic!("Failed to acquire next image: {:?}", e)
                };

                if suboptimal {
                    recreate_swapchain = true;
                }

                let clear_values = vec!([0.0, 0.0, 0.0, 1.0].into(), 1f32.into());
                let mut builder = AutoCommandBufferBuilder::primary_one_time_submit(device.clone(), queue.family()).unwrap();

                builder
                    .begin_render_pass(framebuffers[image_num].clone(), SubpassContents::Inline, clear_values).unwrap()
                    .end_render_pass().unwrap()
                    .draw_text(&mut draw_text, image_num);

                let command_buffer = builder.build().unwrap();

                let future = previous_frame_end.take().unwrap()
                    .join(acquire_future)
                    .then_execute(queue.clone(), command_buffer).unwrap()
                    .then_swapchain_present(queue.clone(), swapchain.clone(), image_num)
                    .then_signal_fence_and_flush();

                match future {
                    Ok(future) => {
                        previous_frame_end = Some(Box::new(future) as Box<_>);
                    },
                    Err(FlushError::OutOfDate) => {
                        recreate_swapchain = true;
                        previous_frame_end = Some(Box::new(sync::now(device.clone())) as Box<_>);
                    }
                    Err(e) => {
                        println!("Failed to flush future: {:?}", e);
                        previous_frame_end = Some(Box::new(sync::now(device.clone())) as Box<_>);
                    }
                }

                frames_rendered += 1;
                if let Some(max_frames) = benchmark_count {
                    if frames_rendered >= max_frames {
                        let duration = start.elapsed();
                        println!("Total Duration: {:?}", duration);
                        println!("Average render Duration: {:?}", duration / frames_rendered as u32);
                        *control_flow = ControlFlow::Exit;
                    }
                }
            },
            _ => ()
        }
    });
}

