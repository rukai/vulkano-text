#[macro_use] extern crate vulkano;
#[macro_use] extern crate vulkano_shader_derive;
extern crate rusttype;

mod render_pass_desc;

use rusttype::{Font, FontCollection, PositionedGlyph, Scale, Rect, point};
use rusttype::gpu_cache::Cache;

use vulkano::buffer::{CpuAccessibleBuffer, BufferUsage};
use vulkano::command_buffer::{DynamicState, AutoCommandBufferBuilder, CommandBufferBuilder};
use vulkano::descriptor::descriptor_set::{DescriptorPool, SimpleDescriptorSet, SimpleDescriptorSetImg};
use vulkano::descriptor::pipeline_layout::{PipelineLayout, PipelineLayoutDescUnion};
use vulkano::device::{Device, Queue};
use vulkano::format::R8Unorm;
use vulkano::framebuffer::{Subpass, RenderPass, RenderPassDesc};
use vulkano::image::SwapchainImage;
use vulkano::image::immutable::ImmutableImage;
use vulkano::pipeline::blend::Blend;
use vulkano::pipeline::depth_stencil::DepthStencil;
use vulkano::pipeline::input_assembly::InputAssembly;
use vulkano::pipeline::multisample::Multisample;
use vulkano::pipeline::vertex::SingleBufferDefinition;
use vulkano::pipeline::viewport::{ViewportsState, Viewport, Scissor};
use vulkano::pipeline::{GraphicsPipeline, GraphicsPipelineParams};
use vulkano::sampler::{Sampler, Filter, MipmapMode, SamplerAddressMode};
use vulkano::swapchain::Swapchain;

use std::sync::Arc;
use std::time::Duration;
use std::io::Write;

#[derive(Debug, Clone)]
struct Vertex {
    position:     [f32; 2],
    tex_position: [f32; 2],
    color:        [f32; 4]
}
impl_vertex!(Vertex, position, tex_position, color);

mod vs {
    #[derive(VulkanoShader)]
    #[ty = "vertex"]
    #[path = "src/shaders/vertex.glsl"]
    struct Dummy;
}

mod fs {
    #[derive(VulkanoShader)]
    #[ty = "fragment"]
    #[path = "src/shaders/fragment.glsl"]
    struct Dummy;
}

struct TextData<'a> {
    glyphs: Vec<PositionedGlyph<'a>>,
    color:  [f32; 4],
}

pub struct DrawText<'a> {
    font:               Font<'a>,
    // device:             Arc<Device>, // TODO: Just store the Device rather then passing it all the time.
    cache:              Cache,
    cache_width:        usize,
    cache_texture:      Arc<ImmutableImage<R8Unorm>>,
    cache_pixel_buffer: Arc<CpuAccessibleBuffer<[u8]>>,
    set:                Arc<SimpleDescriptorSet<((), SimpleDescriptorSetImg<Arc<ImmutableImage<R8Unorm>>>)>>,
    pipeline:           Arc<GraphicsPipeline<SingleBufferDefinition<Vertex>, PipelineLayout<PipelineLayoutDescUnion<vs::Layout, fs::Layout>>, RenderPass<render_pass_desc::Desc>>>,
    texts:              Vec<TextData<'a>>,
}

impl<'a> DrawText<'a> {
    pub fn new(device: Arc<Device>, queue: Arc<Queue>, swapchain: Arc<Swapchain>, images: &[Arc<SwapchainImage>]) -> DrawText<'a> {
        let font_data = include_bytes!("DejaVuSans.ttf");
        let collection = FontCollection::from_bytes(font_data as &[u8]);
        let font = collection.into_font().unwrap();

        let vs = vs::Shader::load(&device).unwrap();
        let fs = fs::Shader::load(&device).unwrap();

        let (cache_width, cache_height) = (1024, 1024);
        let cache = Cache::new(cache_width as u32, cache_height as u32, 0.1, 0.1);

        let cache_pixel_buffer = CpuAccessibleBuffer::<[u8]>::from_iter(
            device.clone(),
            BufferUsage::all(),
            Some(queue.family()),
            (0..cache_width * cache_height).map(|_| 0)
        ).unwrap();

        let cache_texture = vulkano::image::immutable::ImmutableImage::new(
            device.clone(),
            vulkano::image::Dimensions::Dim2d { width: cache_width as u32, height: cache_height as u32 },
            vulkano::format::R8Unorm,
            Some(queue.family())
        ).unwrap();

        let sampler = Sampler::new(
            device.clone(),
            Filter::Linear,
            Filter::Linear,
            MipmapMode::Nearest,
            SamplerAddressMode::Repeat,
            SamplerAddressMode::Repeat,
            SamplerAddressMode::Repeat,
            0.0, 1.0, 0.0, 0.0
        ).unwrap();

        let render_pass = render_pass_desc::Desc { // TODO: panic here
            color: (swapchain.format(), 1)
        }.build_render_pass(device.clone()).unwrap();

        let pipeline = Arc::new(GraphicsPipeline::new(device.clone(), GraphicsPipelineParams {
            vertex_input:    SingleBufferDefinition::new(),
            vertex_shader:   vs.main_entry_point(),
            input_assembly:  InputAssembly::triangle_list(),
            tessellation:    None,
            geometry_shader: None,
            viewport: ViewportsState::Fixed {
                data: vec![(
                    Viewport {
                        origin:      [0.0, 0.0],
                        depth_range: 0.0 .. 1.0,
                        dimensions:  [images[0].dimensions()[0] as f32, images[0].dimensions()[1] as f32],
                    },
                    Scissor::irrelevant()
                )],
            },
            raster:          Default::default(),
            multisample:     Multisample::disabled(),
            fragment_shader: fs.main_entry_point(),
            depth_stencil:   DepthStencil::disabled(),
            blend:           Blend::alpha_blending(),
            render_pass:     Subpass::from(render_pass, 0).unwrap(),
        }).unwrap());

        let set = Arc::new(simple_descriptor_set!(pipeline.clone(), 0, {
            tex: (cache_texture.clone(), sampler.clone())
        }));

        DrawText {
            font:               font,
            cache:              cache,
            cache_width:        cache_width,
            cache_texture:      cache_texture,
            cache_pixel_buffer: cache_pixel_buffer,
            set:                set,
            pipeline:           pipeline,
            texts:              vec!(),
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

    pub fn update_cache(&mut self, command_buffer: AutoCommandBufferBuilder) -> AutoCommandBufferBuilder {
        // Use these as references to make the borrow checker happy
        let cache_pixel_buffer = &mut self.cache_pixel_buffer;
        let cache = &mut self.cache;
        let cache_width = self.cache_width;

        cache.cache_queued(
            |rect, tex_data| {
                let width = (rect.max.x - rect.min.x) as usize;
                let height = (rect.max.y - rect.min.y) as usize;
                let mut cache_lock = cache_pixel_buffer.write().unwrap();
                let mut cache_location = rect.min.y as usize * cache_width + rect.min.x as usize;

                for line in 0..height {
                    let mut cache_slice = &mut cache_lock[cache_location..];
                    let start = line * width;
                    let end   = (line + 1) * width;
                    cache_slice.write(&tex_data[start..end]).unwrap();
                    cache_location += cache_width;
                }
            }
        ).unwrap();

        command_buffer.copy_buffer_to_image(
            cache_pixel_buffer.clone(),
            self.cache_texture.clone(),
        ).unwrap()
    }

    pub fn draw_text(&mut self, mut command_buffer_draw: AutoCommandBufferBuilder, device: Arc<Device>, queue: Arc<Queue>, screen_width: u32, screen_height: u32) -> AutoCommandBufferBuilder {
        let cache = &mut self.cache;
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

            let vertex_buffer = CpuAccessibleBuffer::from_iter(device.clone(), BufferUsage::all(), Some(queue.family()), vertices.into_iter()).unwrap();
            command_buffer_draw = command_buffer_draw.draw(self.pipeline.clone(), DynamicState::none(), vertex_buffer.clone(), self.set.clone(), ()).unwrap();
        }
        command_buffer_draw
    }
}

impl UpdateTextCache for AutoCommandBufferBuilder {
    fn update_text_cache(self, data: &mut DrawText) -> AutoCommandBufferBuilder {
        data.update_cache(self)
    }
}

impl DrawTextTrait for AutoCommandBufferBuilder {
    fn draw_text(self, data: &mut DrawText, device: Arc<Device>, queue: Arc<Queue>, screen_width: u32, screen_height: u32) -> AutoCommandBufferBuilder {
        data.draw_text(self, device, queue, screen_width, screen_height)
    }
}

pub trait UpdateTextCache {
    fn update_text_cache(self, data: &mut DrawText) -> AutoCommandBufferBuilder;
}

pub trait DrawTextTrait {
    fn draw_text(self, data: &mut DrawText, device: Arc<Device>, queue: Arc<Queue>, screen_width: u32, screen_height: u32) -> AutoCommandBufferBuilder;
}
