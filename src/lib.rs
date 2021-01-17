use rusttype::{Font, PositionedGlyph, Scale, Rect, point};
use rusttype::gpu_cache::Cache;

use vulkano::buffer::{CpuAccessibleBuffer, BufferUsage};
use vulkano::command_buffer::{DynamicState, AutoCommandBufferBuilder, SubpassContents};
use vulkano::descriptor::descriptor_set::{PersistentDescriptorSet};
use vulkano::descriptor::pipeline_layout::PipelineLayoutAbstract;
use vulkano::device::{Device, Queue};
use vulkano::format::{R8Unorm, ClearValue};
use vulkano::framebuffer::{Framebuffer, FramebufferAbstract, Subpass, RenderPassAbstract};
use vulkano::image::{SwapchainImage, ImmutableImage, ImageUsage, ImageLayout, Dimensions};
use vulkano::pipeline::vertex::SingleBufferDefinition;
use vulkano::pipeline::viewport::Viewport;
use vulkano::pipeline::GraphicsPipeline;
use vulkano::sampler::{Sampler, Filter, MipmapMode, SamplerAddressMode};
use vulkano::swapchain::Swapchain;

use std::iter;
use std::sync::Arc;

#[derive(Default, Debug, Clone)]
struct Vertex {
    position:     [f32; 2],
    tex_position: [f32; 2],
    color:        [f32; 4]
}
vulkano::impl_vertex!(Vertex, position, tex_position, color);

mod vs {
    vulkano_shaders::shader!{
        ty: "vertex",
        path: "src/shaders/vertex.glsl",
    }
}

mod fs {
    vulkano_shaders::shader!{
        ty: "fragment",
        path: "src/shaders/fragment.glsl",
    }
}

struct TextData {
    glyphs: Vec<PositionedGlyph<'static>>,
    color:  [f32; 4],
}

pub struct DrawText {
    device:             Arc<Device>,
    queue:              Arc<Queue>,
    font:               Font<'static>,
    cache:              Cache<'static>,
    cache_pixel_buffer: Vec<u8>,
    pipeline:           Arc<GraphicsPipeline<SingleBufferDefinition<Vertex>, Box<dyn PipelineLayoutAbstract + Send + Sync>, Arc<dyn RenderPassAbstract + Send + Sync>>>,
    framebuffers:       Vec<Arc<dyn FramebufferAbstract + Send + Sync>>,
    texts:              Vec<TextData>,
}

const CACHE_WIDTH: usize = 1000;
const CACHE_HEIGHT: usize = 1000;

impl DrawText {
    pub fn new<W>(device: Arc<Device>, queue: Arc<Queue>, swapchain: Arc<Swapchain<W>>, images: &[Arc<SwapchainImage<W>>]) -> DrawText where W: Send + Sync + 'static {
        let font_data = include_bytes!("DejaVuSans.ttf");
        let font = Font::from_bytes(font_data as &[u8]).unwrap();

        let vs = vs::Shader::load(device.clone()).unwrap();
        let fs = fs::Shader::load(device.clone()).unwrap();

        let cache = Cache::builder().dimensions(CACHE_WIDTH as u32, CACHE_HEIGHT as u32).build();
        let cache_pixel_buffer = vec!(0; CACHE_WIDTH * CACHE_HEIGHT);

        let render_pass = Arc::new(vulkano::single_pass_renderpass!(device.clone(),
            attachments: {
                color: {
                    load: Load,
                    store: Store,
                    format: swapchain.format(),
                    samples: 1,
                }
            },
            pass: {
                color: [color],
                depth_stencil: {}
            }
        ).unwrap()) as Arc<dyn RenderPassAbstract + Send + Sync>;

        let framebuffers = images.iter().map(|image| {
            Arc::new(
                Framebuffer::start(render_pass.clone())
                .add(image.clone()).unwrap()
                .build().unwrap()
            ) as Arc<dyn FramebufferAbstract + Send + Sync>
        }).collect::<Vec<_>>();

        let pipeline = Arc::new(GraphicsPipeline::start()
            .vertex_input_single_buffer()
            .vertex_shader(vs.main_entry_point(), ())
            .triangle_list()
            .viewports(iter::once(Viewport {
                origin:      [0.0, 0.0],
                depth_range: 0.0..1.0,
                dimensions:  [
                    images[0].dimensions()[0] as f32,
                    images[0].dimensions()[1] as f32
                ],
            }))
            .fragment_shader(fs.main_entry_point(), ())
            .blend_alpha_blending()
            .render_pass(Subpass::from(render_pass.clone(), 0).unwrap())
            .build(device.clone())
            .unwrap()
        );

        DrawText {
            device: device.clone(),
            queue,
            font,
            cache,
            cache_pixel_buffer,
            pipeline,
            framebuffers,
            texts: vec!(),
        }
    }

    pub fn queue_text(&mut self, x: f32, y: f32, size: f32, color: [f32; 4], text: &str) {
        let glyphs: Vec<PositionedGlyph> = self.font.layout(text, Scale::uniform(size), point(x, y)).map(|x| x.standalone()).collect();
        for glyph in &glyphs {
            self.cache.queue_glyph(0, glyph.clone());
        }
        self.texts.push(TextData {
            glyphs: glyphs.clone(),
            color:  color,
        });
    }

    pub fn draw_text<'a>(&mut self, command_buffer: &'a mut AutoCommandBufferBuilder, image_num: usize) -> &'a mut AutoCommandBufferBuilder {
        let screen_width  = self.framebuffers[image_num].dimensions()[0];
        let screen_height = self.framebuffers[image_num].dimensions()[1];
        let cache_pixel_buffer = &mut self.cache_pixel_buffer;
        let cache = &mut self.cache;

        // update texture cache
        cache.cache_queued(
            |rect, src_data| {
                let width = (rect.max.x - rect.min.x) as usize;
                let height = (rect.max.y - rect.min.y) as usize;
                let mut dst_index = rect.min.y as usize * CACHE_WIDTH + rect.min.x as usize;
                let mut src_index = 0;

                for _ in 0..height {
                    let dst_slice = &mut cache_pixel_buffer[dst_index..dst_index+width];
                    let src_slice = &src_data[src_index..src_index+width];
                    dst_slice.copy_from_slice(src_slice);

                    dst_index += CACHE_WIDTH;
                    src_index += width;
                }
            }
        ).unwrap();

        let buffer = CpuAccessibleBuffer::<[u8]>::from_iter(
            self.device.clone(),
            BufferUsage::all(),
            false,
            cache_pixel_buffer.iter().cloned()
        ).unwrap();

        let (cache_texture, cache_texture_write) = ImmutableImage::uninitialized(
            self.device.clone(),
            Dimensions::Dim2d { width: CACHE_WIDTH as u32, height: CACHE_HEIGHT as u32 },
            R8Unorm,
            1,
            ImageUsage {
                sampled: true,
                transfer_destination: true,
                .. ImageUsage::none()
            },
            ImageLayout::General,
            Some(self.queue.family())
        ).unwrap();

        let sampler = Sampler::new(
            self.device.clone(),
            Filter::Linear,
            Filter::Linear,
            MipmapMode::Nearest,
            SamplerAddressMode::Repeat,
            SamplerAddressMode::Repeat,
            SamplerAddressMode::Repeat,
            0.0, 1.0, 0.0, 0.0
        ).unwrap();

        let set = Arc::new(
            PersistentDescriptorSet::start(self.pipeline.descriptor_set_layout(0).unwrap().clone())
            .add_sampled_image(cache_texture.clone(), sampler).unwrap()
            .build().unwrap()
        );

        let mut command_buffer = command_buffer
            .copy_buffer_to_image(
                buffer.clone(),
                cache_texture_write,
            ).unwrap()
            .begin_render_pass(self.framebuffers[image_num].clone(), SubpassContents::Inline, vec!(ClearValue::None)).unwrap();

        // draw
        for text in &mut self.texts.drain(..) {
            let vertices: Vec<Vertex> = text.glyphs.iter().flat_map(|g| {
                if let Ok(Some((uv_rect, screen_rect))) = cache.rect_for(0, g) {
                    let gl_rect = Rect {
                        min: point(
                            (screen_rect.min.x as f32 / screen_width  as f32 - 0.5) * 2.0,
                            (screen_rect.min.y as f32 / screen_height as f32 - 0.5) * 2.0
                        ),
                        max: point(
                           (screen_rect.max.x as f32 / screen_width  as f32 - 0.5) * 2.0,
                           (screen_rect.max.y as f32 / screen_height as f32 - 0.5) * 2.0
                        )
                    };
                    vec!(
                        Vertex {
                            position:     [gl_rect.min.x, gl_rect.max.y],
                            tex_position: [uv_rect.min.x, uv_rect.max.y],
                            color:        text.color,
                        },
                        Vertex {
                            position:     [gl_rect.min.x, gl_rect.min.y],
                            tex_position: [uv_rect.min.x, uv_rect.min.y],
                            color:        text.color,
                        },
                        Vertex {
                            position:     [gl_rect.max.x, gl_rect.min.y],
                            tex_position: [uv_rect.max.x, uv_rect.min.y],
                            color:        text.color,
                        },

                        Vertex {
                            position:     [gl_rect.max.x, gl_rect.min.y],
                            tex_position: [uv_rect.max.x, uv_rect.min.y],
                            color:        text.color,
                        },
                        Vertex {
                            position:     [gl_rect.max.x, gl_rect.max.y],
                            tex_position: [uv_rect.max.x, uv_rect.max.y],
                            color:        text.color,
                        },
                        Vertex {
                            position:     [gl_rect.min.x, gl_rect.max.y],
                            tex_position: [uv_rect.min.x, uv_rect.max.y],
                            color:        text.color,
                        },
                    ).into_iter()
                }
                else {
                    vec!().into_iter()
                }
            }).collect();

            let vertex_buffer = CpuAccessibleBuffer::from_iter(self.device.clone(), BufferUsage::all(), false, vertices.into_iter()).unwrap();
            command_buffer = command_buffer.draw(self.pipeline.clone(), &DynamicState::none(), vertex_buffer.clone(), set.clone(), ()).unwrap();
        }

        command_buffer.end_render_pass().unwrap()
    }
}

impl DrawTextTrait for AutoCommandBufferBuilder {
    fn draw_text(&mut self, data: &mut DrawText, image_num: usize) -> &mut Self {
        data.draw_text(self, image_num)
    }
}

pub trait DrawTextTrait {
    fn draw_text(&mut self, data: &mut DrawText, image_num: usize) -> &mut Self;
}
