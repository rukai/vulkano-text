#[macro_use] extern crate vulkano;
extern crate rusttype;

use rusttype::{
    Font,
    FontCollection,
    PositionedGlyph,
    Scale,
    Rect,
    point,
};
use rusttype::gpu_cache::Cache;

use vulkano::buffer::{CpuAccessibleBuffer, BufferUsage};
use vulkano::command_buffer::{PrimaryCommandBufferBuilder, PrimaryCommandBufferBuilderInlineDraw, DynamicState};
use vulkano::device::{Device, Queue};
use vulkano::framebuffer::Subpass;
use vulkano::image::SwapchainImage;
use vulkano::pipeline::blend::Blend;
use vulkano::pipeline::depth_stencil::DepthStencil;
use vulkano::pipeline::input_assembly::InputAssembly;
use vulkano::pipeline::multisample::Multisample;
use vulkano::pipeline::vertex::SingleBufferDefinition;
use vulkano::pipeline::viewport::{ViewportsState, Viewport, Scissor};
use vulkano::pipeline::{GraphicsPipeline, GraphicsPipelineParams};
use vulkano::descriptor::descriptor_set::DescriptorPool;
use vulkano::sampler::{Sampler, Filter, MipmapMode, SamplerAddressMode};
use vulkano::image::immutable::ImmutableImage;
use vulkano::format::R8Unorm;

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

mod vs { include!{concat!(env!("OUT_DIR"), "/shaders/src/shaders/vertex.glsl")} }
mod fs { include!{concat!(env!("OUT_DIR"), "/shaders/src/shaders/fragment.glsl")} }

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

mod pipeline_layout {
    pipeline_layout!{
        set0: {
            tex: CombinedImageSampler
        }
    }
}

struct TextData<'a> {
    glyphs: Vec<PositionedGlyph<'a>>,
    color:  [f32; 4],
}

pub struct DrawText<'a> {
    font:               Font<'a>,
    cache:              Cache,
    cache_width:        usize,
    cache_texture:      Arc<ImmutableImage<R8Unorm>>,
    cache_pixel_buffer: Arc<CpuAccessibleBuffer<[u8]>>,
    set:                Arc<pipeline_layout::set0::Set>,
    pipeline:           Arc<GraphicsPipeline<SingleBufferDefinition<Vertex>, pipeline_layout::CustomPipeline, render_pass::CustomRenderPass>>,
    texts:              Vec<TextData<'a>>,
}

impl<'a> DrawText<'a> {
    pub fn new(device: &Arc<Device>, queue: &Arc<Queue>, images: &Vec<Arc<SwapchainImage>>) -> DrawText<'a> {
        let font_data = include_bytes!("DejaVuSans.ttf");
        let collection = FontCollection::from_bytes(font_data as &[u8]);
        let font = collection.into_font().unwrap();

        let vs = vs::Shader::load(&device).unwrap();
        let fs = fs::Shader::load(&device).unwrap();

        let render_pass = render_pass::CustomRenderPass::new(&device, &render_pass::Formats {
            color: (images[0].format(), 1)
        }).unwrap();

        let (cache_width, cache_height) = (1024, 1024);
        let cache = Cache::new(cache_width as u32, cache_height as u32, 0.1, 0.1);

        let cache_pixel_buffer = CpuAccessibleBuffer::<[u8]>::from_iter(
            device,
            &BufferUsage::all(),
            Some(queue.family()),
            (0..cache_width * cache_height).map(|_| 0)
        ).unwrap();

        let cache_texture = vulkano::image::immutable::ImmutableImage::new(
            &device,
            vulkano::image::Dimensions::Dim2d { width: cache_width as u32, height: cache_height as u32 },
            vulkano::format::R8Unorm,
            Some(queue.family())
        ).unwrap();

        let sampler = Sampler::new(
            &device,
            Filter::Linear,
            Filter::Linear,
            MipmapMode::Nearest,
            SamplerAddressMode::Repeat,
            SamplerAddressMode::Repeat,
            SamplerAddressMode::Repeat,
            0.0, 1.0, 0.0, 0.0
        ).unwrap();

        let descriptor_pool = DescriptorPool::new(&device);
        let pipeline_layout = pipeline_layout::CustomPipeline::new(&device).unwrap();
        let set = pipeline_layout::set0::Set::new(
            &descriptor_pool,
            &pipeline_layout,
            &pipeline_layout::set0::Descriptors {
                tex: (&sampler, &cache_texture)
            }
        );

        let pipeline = GraphicsPipeline::new(&device, GraphicsPipelineParams {
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
                        dimensions:  [images[0].dimensions()[0] as f32,
                                      images[0].dimensions()[1] as f32],
                    },
                    Scissor::irrelevant()
                )],
            },
            raster:          Default::default(),
            multisample:     Multisample::disabled(),
            fragment_shader: fs.main_entry_point(),
            depth_stencil:   DepthStencil::disabled(),
            blend:           Blend::alpha_blending(),
            layout:          &pipeline_layout,
            render_pass:     Subpass::from(&render_pass, 0).unwrap(),
        }).unwrap();

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

    pub fn update_cache(&mut self, command_buffer: PrimaryCommandBufferBuilder) -> PrimaryCommandBufferBuilder {
        // Use these as references to make the borrow checker happy
        let cache_pixel_buffer = &mut self.cache_pixel_buffer;
        let cache = &mut self.cache;
        let cache_width = self.cache_width;

        cache.cache_queued(
            |rect, tex_data| {
                let width = (rect.max.x - rect.min.x) as usize;
                let height = (rect.max.y - rect.min.y) as usize;
                let mut cache_lock = cache_pixel_buffer.write(Duration::new(1, 0)).unwrap();
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

        command_buffer.copy_buffer_to_color_image(
            &(*cache_pixel_buffer),
            &self.cache_texture,
            0,
            0 .. 1,
            [0, 0, 0],
            [
                self.cache_texture.dimensions().width(),
                self.cache_texture.dimensions().height(),
                1
            ]
        )
    }

    pub fn draw_text(&mut self, mut command_buffer_draw: PrimaryCommandBufferBuilderInlineDraw, device: &Arc<Device>, queue: &Arc<Queue>, screen_width: u32, screen_height: u32) -> PrimaryCommandBufferBuilderInlineDraw {
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

            let vertex_buffer = CpuAccessibleBuffer::from_iter(&device, &BufferUsage::all(), Some(queue.family()), vertices.into_iter()).expect("failed to create buffer");
            command_buffer_draw = command_buffer_draw.draw(&self.pipeline, &vertex_buffer, &DynamicState::none(), &self.set, &());
        }
        command_buffer_draw
    }
}

impl UpdateTextCache for PrimaryCommandBufferBuilder {
    fn update_text_cache(self, data: &mut DrawText) -> PrimaryCommandBufferBuilder {
        data.update_cache(self)
    }
}

impl DrawTextTrait for PrimaryCommandBufferBuilderInlineDraw {
    fn draw_text(self, data: &mut DrawText, device: &Arc<Device>, queue: &Arc<Queue>, screen_width: u32, screen_height: u32) -> PrimaryCommandBufferBuilderInlineDraw {
        data.draw_text(self, device, queue, screen_width, screen_height)
    }
}

pub trait UpdateTextCache {
    fn update_text_cache(self, data: &mut DrawText) -> PrimaryCommandBufferBuilder;
}

pub trait DrawTextTrait {
    fn draw_text(self, data: &mut DrawText, device: &Arc<Device>, queue: &Arc<Queue>, screen_width: u32, screen_height: u32) -> PrimaryCommandBufferBuilderInlineDraw;
}
