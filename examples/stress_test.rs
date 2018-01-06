#[macro_use]
extern crate vulkano;
extern crate winit;
extern crate vulkano_win;

extern crate vulkano_text;
use vulkano_text::{DrawText, DrawTextTrait};

use vulkano::command_buffer::AutoCommandBufferBuilder;
use vulkano::device::Device;
use vulkano::format::Format;
use vulkano::framebuffer::Framebuffer;
use vulkano::image::attachment::AttachmentImage;
use vulkano::instance::Instance;
use vulkano::swapchain::PresentMode;
use vulkano::swapchain::SurfaceTransform;
use vulkano::swapchain::Swapchain;
use vulkano::swapchain;
use vulkano::sync::GpuFuture;
use vulkano::sync::now;
use vulkano_win::VkSurfaceBuild;

use std::sync::Arc;
use std::time::Instant;
use std::env;

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

    let instance = {
        let extensions = vulkano_win::required_extensions();
        Instance::new(None, &extensions, None).expect("failed to create Vulkan instance")
    };

    let physical = vulkano::instance::PhysicalDevice::enumerate(&instance).next().expect("no device available");
    let mut events_loop = winit::EventsLoop::new();
    let surface = winit::WindowBuilder::new().build_vk_surface(&events_loop, instance.clone()).unwrap();
    let queue = physical.queue_families().find(|&q| {
        q.supports_graphics() && surface.is_supported(q).unwrap_or(false)
    }).expect("couldn't find a graphical queue family");

    let (device, mut queues) = {
        let device_ext = vulkano::device::DeviceExtensions {
            khr_swapchain: true,
            .. vulkano::device::DeviceExtensions::none()
        };

        Device::new(physical, physical.supported_features(), &device_ext,
                    [(queue, 0.5)].iter().cloned()).expect("failed to create device")
    };

    let queue = queues.next().unwrap();

    let (swapchain, images) = {
        let caps = surface.capabilities(physical)
                         .expect("failed to get surface capabilities");

        let dimensions = caps.current_extent.unwrap_or([1280, 1024]);
        let alpha = caps.supported_composite_alpha.iter().next().unwrap();
        let format = caps.supported_formats[0].0;
        Swapchain::new(device.clone(), surface.clone(), caps.min_image_count, format,
                       dimensions, 1, caps.supported_usage_flags, &queue,
                       SurfaceTransform::Identity, alpha, PresentMode::Fifo, true,
                       None).expect("failed to create swapchain")
    };

    // include a depth buffer (unlike triangle.rs) to ensure vulkano-text isnt dependent on a specific render_pass
    let render_pass = Arc::new(single_pass_renderpass!(device.clone(),
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

    let depthbuffer = AttachmentImage::transient(device.clone(), images[0].dimensions(), Format::D16Unorm).unwrap();
    let framebuffers = images.iter().map(|image| {
        Arc::new(Framebuffer::start(render_pass.clone())
            .add(image.clone()).unwrap()
            .add(depthbuffer.clone()).unwrap()
            .build().unwrap())
    }).collect::<Vec<_>>();

    let mut draw_text = DrawText::new(device.clone(), queue.clone(), swapchain.clone(), &images);

    let (width, _) = surface.window().get_inner_size().unwrap();
    let mut x = 0.0;

    let mut previous_frame_end = Box::new(now(device.clone())) as Box<GpuFuture>;

    let start = Instant::now();
    let mut frames_rendered = 0;
    loop {
        frames_rendered += 1;
        previous_frame_end.cleanup_finished();

        if x > width as f32 {
            x = 0.0;
        }
        else {
            x += 2.0;
        }

        for (i, line) in lines.iter().enumerate() {
            draw_text.queue_text(x, (i + 1) as f32 * 15.0, 15.0, [1.0, 1.0, 1.0, 1.0], line);
        }

        let (image_num, acquire_future) = swapchain::acquire_next_image(swapchain.clone(), None).unwrap();
        let command_buffer = AutoCommandBufferBuilder::new(device.clone(), queue.family()).unwrap()
            .begin_render_pass(framebuffers[image_num].clone(), false, vec![[0.0, 0.0, 0.0, 1.0].into(), 1f32.into()]).unwrap()
            .end_render_pass().unwrap()
            .draw_text(&mut draw_text, image_num)
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
        if done {
            break;
        }
        if let Some(max_frames) = benchmark_count {
            if frames_rendered >= max_frames {
                break;
            }
        }
    }
    let duration = start.elapsed();

    println!("Total Duration: {:?}", duration);
    println!("Average render Duration: {:?}", duration / frames_rendered as u32);
}
