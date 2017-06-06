// Most of this is just standard vulkano setup
// Check the code following the UNIQUE CODE comments

#[macro_use]
extern crate vulkano;
extern crate winit;
extern crate vulkano_win;
extern crate vulkano_text;

use vulkano_text::{DrawText, DrawTextTrait, UpdateTextCache};

use vulkano_win::VkSurfaceBuild;
use vulkano::command_buffer;
use vulkano::command_buffer::PrimaryCommandBufferBuilder;
use vulkano::command_buffer::Submission;
use vulkano::device::Device;
use vulkano::framebuffer::Framebuffer;
use vulkano::instance::Instance;
use vulkano::swapchain::SurfaceTransform;
use vulkano::swapchain::Swapchain;

use std::sync::Arc;
use std::time::Duration;

mod render_pass {
    use vulkano::format::Format;

    single_pass_renderpass!{
        attachments: {
            color: {
                load: Clear,
                store: Store,
                format: Format,
            }
        },
        pass: {
            color: [color],
            depth_stencil: {}
        }
    }
}

fn main() {
    let instance = {
        let extensions = vulkano_win::required_extensions();
        Instance::new(None, &extensions, None).expect("failed to create Vulkan instance")
    };
    let physical = vulkano::instance::PhysicalDevice::enumerate(&instance).next().expect("no device available");
    let window = winit::WindowBuilder::new().build_vk_surface(&instance).unwrap();
    let queue = physical.queue_families().find(|q| {
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
        let caps = window.surface().get_capabilities(&physical)
                         .expect("failed to get surface capabilities");
        let dimensions = caps.current_extent.unwrap_or([1280, 1024]);
        let present = caps.present_modes.iter().next().unwrap();
        let alpha = caps.supported_composite_alpha.iter().next().unwrap();
        let format = caps.supported_formats[0].0;
        Swapchain::new(&device, &window.surface(), caps.min_image_count, format, dimensions, 1,
           &caps.supported_usage_flags, &queue, SurfaceTransform::Identity, alpha,
           present, true, None).expect("failed to create swapchain")
    };

    let render_pass = render_pass::CustomRenderPass::new(&device, &render_pass::Formats {
        color: (images[0].format(), 1)
    }).unwrap();

    let mut framebuffers = images.iter().map(|image| {
        let dimensions = [image.dimensions()[0], image.dimensions()[1], 1];
        Framebuffer::new(&render_pass, dimensions, render_pass::AList {
            color: image
        }).unwrap()
    }).collect::<Vec<_>>();

    let mut submissions: Vec<Arc<Submission>> = Vec::new();

    // UNIQUE CODE: create DrawText
    let mut draw_text = DrawText::new(&device, &queue, &images);

    let mut x = -200.0;

    let (mut width, mut height) = window.window().get_inner_size_points().unwrap();

    loop {
        // TODO: https://github.com/tomaka/vulkano/issues/366 I guess we are waiting on the vulkano rewrite for this to not explode
        // TODO: Comment out all the draw_text code and resizing fails as described in the issue.
        let (new_width, new_height) = window.window().get_inner_size_points().unwrap();
        if width != new_width || height != new_height {
            width = new_width;
            height = new_height;

            let swapchain_results = swapchain.recreate_with_dimension([width, height]).unwrap();
            swapchain = swapchain_results.0;
            images = swapchain_results.1;

            framebuffers = images.iter().map(|image| {
                let dimensions = [image.dimensions()[0], image.dimensions()[1], 1];
                Framebuffer::new(&render_pass, dimensions, render_pass::AList {
                    color: image
                }).unwrap()
            }).collect::<Vec<_>>();

            // UNIQUE CODE: recreate DrawText due to new window size
            draw_text = DrawText::new(&device, &queue, &images);
        }

        if x > width as f32 {
            x = 0.0;
        }
        else {
            x += 0.4;
        }

        // UNIQUE CODE: add queue text
        draw_text.queue_text(200.0, 50.0, 20.0, [1.0, 1.0, 1.0, 1.0], "The quick brown fox jumps over the lazy dog.");
        draw_text.queue_text(20.0, 200.0, 190.0, [1.0, 0.0, 0.0, 1.0], "Hello world!");
        draw_text.queue_text(x, 350.0, 70.0, [0.51, 0.6, 0.74, 1.0], "Lenny: ( ͡° ͜ʖ ͡°)");
        draw_text.queue_text(50.0, 350.0, 70.0, [1.0, 1.0, 1.0, 1.0], "Overlap");

        submissions.retain(|s| s.destroying_would_block());
        let image_num = swapchain.acquire_next_image(Duration::new(1, 0)).unwrap();

        let command_buffer = PrimaryCommandBufferBuilder::new(&device, queue.family())

        // UNIQUE CODE: update DrawTextData internal cache
        .update_text_cache(&mut draw_text)

        .draw_inline(&render_pass, &framebuffers[image_num], render_pass::ClearValues {
            color: [0.0, 0.0, 0.0, 1.0]
        })

        // UNIQUE CODE: draw the text
        .draw_text(&mut draw_text, &device, &queue, width, height)

        .draw_end()
        .build();

        submissions.push(command_buffer::submit(&command_buffer, &queue).unwrap());
        swapchain.present(&queue, image_num).unwrap();

        for ev in window.window().poll_events() {
            match ev {
                winit::Event::Closed => return,
                _ => ()
            }
        }
    }
}
