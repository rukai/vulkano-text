// Most of this is just standard vulkano setup
// Check the code between UNIQUE CODE comments

#[macro_use]
extern crate vulkano;
extern crate winit;
extern crate vulkano_win;
extern crate vulkano_text;

use vulkano_text::{DrawText, DrawTextTrait, UpdateTextCache};

use vulkano_win::VkSurfaceBuild;
use vulkano::command_buffer::{CommandBufferBuilder, AutoCommandBufferBuilder};
use vulkano::device::Device;
use vulkano::framebuffer::Framebuffer;
use vulkano::instance::Instance;
use vulkano::swapchain::{Swapchain, SurfaceTransform, PresentMode};
use vulkano::swapchain;
use vulkano::sync::{now, GpuFuture};

use std::sync::Arc;
use std::time::Duration;

fn main() {
    let instance = {
        let extensions = vulkano_win::required_extensions();
        Instance::new(None, &extensions, None).expect("failed to create Vulkan instance")
    };
    let physical = vulkano::instance::PhysicalDevice::enumerate(&instance).next().expect("no device available");
    let events_loop = winit::EventsLoop::new();
    let window = winit::WindowBuilder::new().build_vk_surface(&events_loop, instance.clone()).unwrap();
    let queue = physical.queue_families().find(|&q| {
        q.supports_graphics() && window.surface().is_supported(q).unwrap_or(false)
    }).expect("couldn't find a graphical queue family");

    let (device, mut queues) = {
        let device_ext = vulkano::device::DeviceExtensions {
            khr_swapchain: true,
            .. vulkano::device::DeviceExtensions::none()
        };

        Device::new(&physical, physical.supported_features(), &device_ext,
                    [(queue, 0.5)].iter().cloned()).expect("failed to create device")
    };

    let queue = queues.next().unwrap();
    let (mut swapchain, mut images) = {
        let caps = window.surface().capabilities(physical)
                         .expect("failed to get surface capabilities");
        let dimensions = caps.current_extent.unwrap_or([1280, 1024]);
        let alpha = caps.supported_composite_alpha.iter().next().unwrap();
        let format = caps.supported_formats[0].0;
        Swapchain::new(device.clone(), window.surface().clone(), caps.min_image_count, format, dimensions, 1,
           caps.supported_usage_flags, &queue, SurfaceTransform::Identity, alpha,
           PresentMode::Fifo, true, None
        ).expect("failed to create swapchain")
    };

    let render_pass = Arc::new(single_pass_renderpass!(device.clone(),
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
    ).unwrap());

    let mut framebuffers = images.iter().map(|image| {
        Arc::new(Framebuffer::start(render_pass.clone())
            .add(image.clone()).unwrap()
            .build().unwrap())
    }).collect::<Vec<_>>();

    // UNIQUE CODE: create DrawText
    let mut draw_text = DrawText::new(device.clone(), queue.clone(), swapchain.clone(), &images); // uncommenting causes panic

    let mut x = -200.0;
    // UNIQUE CODE END

    let (mut width, mut height) = window.window().get_inner_size_points().unwrap();

    let mut previous_frame_end = Box::new(now(device.clone())) as Box<GpuFuture>;

    loop {
        previous_frame_end.cleanup_finished();

        let (new_width, new_height) = window.window().get_inner_size_points().unwrap();
        if width != new_width || height != new_height {
            width = new_width;
            height = new_height;

            let swapchain_results = swapchain.recreate_with_dimension([width, height]).unwrap();
            swapchain = swapchain_results.0;
            images = swapchain_results.1;

            framebuffers = images.iter().map(|image| {
                Arc::new(Framebuffer::start(render_pass.clone())
                    .add(image.clone()).unwrap()
                    .build().unwrap())
            }).collect::<Vec<_>>();

            // UNIQUE CODE: recreate DrawText due to new window size
            draw_text = DrawText::new(device.clone(), queue.clone(), swapchain.clone(), &images);
            // UNIQUE CODE END
        }

        // UNIQUE CODE: scrolling text!
        if x > width as f32 {
            x = 0.0;
        }
        else {
            x += 0.4;
        }

        draw_text.queue_text(200.0, 50.0, 20.0, [1.0, 1.0, 1.0, 1.0], "The quick brown fox jumps over the lazy dog.");
        draw_text.queue_text(20.0, 200.0, 190.0, [1.0, 0.0, 0.0, 1.0], "Hello world!");
        draw_text.queue_text(x, 350.0, 70.0, [0.51, 0.6, 0.74, 1.0], "Lenny: ( ͡° ͜ʖ ͡°)");
        draw_text.queue_text(50.0, 350.0, 70.0, [1.0, 1.0, 1.0, 1.0], "Overlap");
        // UNIQUE CODE END

        let (image_num, acquire_future) = swapchain::acquire_next_image(swapchain.clone(), Duration::new(1, 0)).unwrap();

        let command_buffer = AutoCommandBufferBuilder::new(device.clone(), queue.family()).unwrap()
            // UNIQUE CODE: update DrawTextData internal cache
            .update_text_cache(&mut draw_text)
            // UNIQUE CODE END

            .begin_render_pass(framebuffers[image_num].clone(), false, vec![[0.0, 0.0, 0.0, 1.0].into()]).unwrap()

            // UNIQUE CODE: draw the text
            .draw_text(&mut draw_text, queue.clone(), width, height)
            // UNIQUE CODE END

            .end_render_pass().unwrap()
            .build().unwrap();


        let future = previous_frame_end.join(acquire_future)
            .then_execute(queue.clone(), command_buffer).unwrap()
            .then_swapchain_present(queue.clone(), swapchain.clone(), image_num)
            .then_signal_fence_and_flush().unwrap();
        previous_frame_end = Box::new(future) as Box<_>;

        let mut done = false;
        events_loop.poll_events(|ev| {
            match ev {
                winit::Event::WindowEvent { event: winit::WindowEvent::Closed, .. } => done = true,
                _ => ()
            }
        });
        if done { return; }
    }
}
