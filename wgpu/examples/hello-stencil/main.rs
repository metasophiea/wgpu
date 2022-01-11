use std::borrow::Cow;
use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::Window,
};
use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    position: [f32; 2],
}
impl Vertex {
    pub fn new(position:[f32; 2]) -> Vertex {
        Vertex {
            position,
        }
    }
}
impl Vertex {
    pub fn get_x(&self) -> f32 { self.position[0] }
    pub fn get_y(&self) -> f32 { self.position[1] }
}
impl Vertex {
    pub fn desc<'a>() -> wgpu::VertexBufferLayout<'a> { 
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x2,
                    offset: 0,
                    shader_location: 0,
                },
            ],
        }
    }
}

async fn run(event_loop: EventLoop<()>, window: Window) {
    let size = window.inner_size();
    let instance = wgpu::Instance::new(wgpu::Backends::all());
    let surface = unsafe { instance.create_surface(&window) };
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::default(),
            force_fallback_adapter: false,
            // Request an adapter which can render to our surface
            compatible_surface: Some(&surface),
        })
        .await
        .expect("Failed to find an appropriate adapter");

    //Create the logical device and command queue
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: None,
                    features: wgpu::Features::empty(),
                    // Make sure we use the texture resolution limits from the adapter, so we can support images the size of the swapchain.
                    limits: wgpu::Limits::downlevel_webgl2_defaults()
                        .using_resolution(adapter.limits()),
                },
                None,
            )
            .await
            .expect("Failed to create device");

    //configure surface
        let swapchain_format = surface.get_preferred_format(&adapter).unwrap();
        let mut config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: swapchain_format,
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Mailbox,
        };
        surface.configure(&device, &config);

    //generate framebuffers
        //texture extent
            let texture_extent = wgpu::Extent3d {
                width: size.width,
                height: size.height,
                depth_or_array_layers: 1,
            };
        //colour framebuffer
            let multisampled_frame_descriptor = &wgpu::TextureDescriptor {
                size: texture_extent,
                mip_level_count: 1,
                sample_count: 4,
                dimension: wgpu::TextureDimension::D2,
                format: swapchain_format,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
                label: Some("multisampled colour framebuffer"),
            };
            let multisampled_framebuffer_for_colour = device.create_texture(multisampled_frame_descriptor).create_view(&wgpu::TextureViewDescriptor::default());
        //stencil framebuffer
            let multisampled_frame_descriptor = &wgpu::TextureDescriptor {
                size: texture_extent,
                mip_level_count: 1,
                sample_count: 4,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Depth24PlusStencil8,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
                label: Some("multisampled stencil framebuffer"),
            };
            let multisampled_framebuffer_for_stencil = device.create_texture(multisampled_frame_descriptor).create_view(&wgpu::TextureViewDescriptor::default());

    //load shader
        let shader = device.create_shader_module(&wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("shader.wgsl"))),
        });

    //generate pipelines
        let pipeline_layout = device.create_pipeline_layout(
            &wgpu::PipelineLayoutDescriptor {
                label: None,
                bind_group_layouts: &[],
                push_constant_ranges: &[],
            }
        );
        //regular
            fn create_regular_shader(
                device: &wgpu::Device,
                pipeline_layout: &wgpu::PipelineLayout,
                shader: &wgpu::ShaderModule,
                swapchain_format: &wgpu::TextureFormat,
                fragment_entry_point: &str,
            ) -> wgpu::RenderPipeline {
                device.create_render_pipeline(
                    &wgpu::RenderPipelineDescriptor {
                        label: None,
                        layout: Some(&pipeline_layout),
                        vertex: wgpu::VertexState {
                            module: &shader,
                            entry_point: "vs_main",
                            buffers: &[Vertex::desc()],
                        },
                        fragment: Some(wgpu::FragmentState {
                            module: shader,
                            entry_point: fragment_entry_point,
                            targets: &[wgpu::ColorTargetState {
                                format: *swapchain_format,
                                blend: Some(wgpu::BlendState {
                                    color: wgpu::BlendComponent {
                                        src_factor: wgpu::BlendFactor::One,
                                        dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                                        operation: wgpu::BlendOperation::Add,
                                    },
                                    alpha: wgpu::BlendComponent::REPLACE,
                                }),
                                write_mask: wgpu::ColorWrites::ALL,
                            }],
                        }),
                        primitive: wgpu::PrimitiveState {
                            topology: wgpu::PrimitiveTopology::TriangleList,
                            strip_index_format: None,
                            front_face: wgpu::FrontFace::Cw,
                            cull_mode: Some(wgpu::Face::Back),
                            unclipped_depth: false,
                            polygon_mode: wgpu::PolygonMode::Fill,
                            conservative: false,
                        },
                        depth_stencil: Some(wgpu::DepthStencilState {
                            format: wgpu::TextureFormat::Depth24PlusStencil8,
                            depth_write_enabled: false,
                            depth_compare: wgpu::CompareFunction::Always,
                            stencil: wgpu::StencilState {
                                front: wgpu::StencilFaceState {
                                    compare: wgpu::CompareFunction::Equal,
                                    fail_op: wgpu::StencilOperation::Zero, //only compare matters
                                    depth_fail_op: wgpu::StencilOperation::Zero, //only compare matters
                                    pass_op: wgpu::StencilOperation::Zero, //only compare matters
                                },
                                back: wgpu::StencilFaceState::IGNORE,
                                read_mask: 0xff,
                                write_mask: 0x00,
                            },
                            bias: wgpu::DepthBiasState {
                                constant: 0,
                                slope_scale: 0.0,
                                clamp: 0.0,
                            },
                        }),
                        multisample: wgpu::MultisampleState {
                            count: 4,
                            mask: !0,
                            alpha_to_coverage_enabled: false,
                        },
                        multiview: None,
                    }
                )
            }
            
            let regular_red = create_regular_shader(
                &device,
                &pipeline_layout,
                &shader,
                &swapchain_format,
                "fs_main_red",
            );
            let regular_green = create_regular_shader(
                &device,
                &pipeline_layout,
                &shader,
                &swapchain_format,
                "fs_main_green",
            );
            let regular_blue = create_regular_shader(
                &device,
                &pipeline_layout,
                &shader,
                &swapchain_format,
                "fs_main_blue",
            );
        //increment stencil
            let increment_write_to_stencil = device.create_render_pipeline(
                &wgpu::RenderPipelineDescriptor {
                    label: None,
                    layout: Some(&pipeline_layout),
                    vertex: wgpu::VertexState {
                        module: &shader,
                        entry_point: "vs_main",
                        buffers: &[Vertex::desc()],
                    },
                    fragment: Some(wgpu::FragmentState {
                        module: &shader,
                        entry_point: "fs_main_black",
                        targets: &[wgpu::ColorTargetState {
                            format: swapchain_format,
                            blend: Some(wgpu::BlendState {
                                color: wgpu::BlendComponent {
                                    src_factor: wgpu::BlendFactor::One,
                                    dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                                    operation: wgpu::BlendOperation::Add,
                                },
                                alpha: wgpu::BlendComponent::REPLACE,
                            }),
                            write_mask: wgpu::ColorWrites::empty(),
                        }],
                    }),
                    primitive: wgpu::PrimitiveState::default(),
                    depth_stencil: Some(wgpu::DepthStencilState {
                        format: wgpu::TextureFormat::Depth24PlusStencil8,
                        depth_write_enabled: false,
                        depth_compare: wgpu::CompareFunction::Always,
                        stencil: wgpu::StencilState {
                            front: wgpu::StencilFaceState {
                                compare: wgpu::CompareFunction::Always,
                                fail_op: wgpu::StencilOperation::Keep,
                                depth_fail_op: wgpu::StencilOperation::Keep,
                                pass_op: wgpu::StencilOperation::IncrementClamp,
                            },
                            back: wgpu::StencilFaceState {
                                compare: wgpu::CompareFunction::Always,
                                fail_op: wgpu::StencilOperation::Keep,
                                depth_fail_op: wgpu::StencilOperation::Keep,
                                pass_op: wgpu::StencilOperation::IncrementClamp,
                            },
                            read_mask: 0x00,
                            write_mask: 0xff,
                        },
                        bias: wgpu::DepthBiasState {
                            constant: 0,
                            slope_scale: 0.0,
                            clamp: 0.0,
                        },
                    }),
                    multisample: wgpu::MultisampleState {
                        count: 4,
                        mask: !0,
                        alpha_to_coverage_enabled: false,
                    },
                    multiview: None,
                }
            );

    //activate event_loop
        event_loop.run(move |event, _, control_flow| {
            // Have the closure take ownership of the resources.
            // `event_loop.run` never returns, therefore we must do this to ensure
            // the resources are properly cleaned up.
            let _ = (&instance, &adapter, &shader, &pipeline_layout);
    
            *control_flow = ControlFlow::Wait;
            match event {
                Event::WindowEvent {
                    event: WindowEvent::Resized(size),
                    ..
                } => {
                    // Reconfigure the surface with the new size
                    config.width = size.width;
                    config.height = size.height;
                    surface.configure(&device, &config);
                }
                Event::RedrawRequested(_) => {
                    let frame = surface.get_current_texture().expect("Failed to acquire next swap chain texture");
                    let frame_texture_view = frame.texture.create_view( &wgpu::TextureViewDescriptor::default() );
                    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

                    //stencil triangle
                        let vertex_points_stencil:Vec<Vertex> = vec![
                            Vertex::new([-0.5, -0.5]),
                            Vertex::new([0.0, 0.5]),
                            Vertex::new([0.5, -0.5]),
                        ];
                        let vertex_buffer_stencil = device.create_buffer_init(
                            &wgpu::util::BufferInitDescriptor {
                                label: None,
                                contents: bytemuck::cast_slice(&vertex_points_stencil),
                                usage: wgpu::BufferUsages::VERTEX,
                            }
                        );
                    //red triangle
                        let vertex_points_red:Vec<Vertex> = vec![
                            Vertex::new([-0.75, -0.5]),
                            Vertex::new([-0.25, 0.5]),
                            Vertex::new([0.25, -0.5]),
                        ];
                        let vertex_buffer_red = device.create_buffer_init(
                            &wgpu::util::BufferInitDescriptor {
                                label: None,
                                contents: bytemuck::cast_slice(&vertex_points_red),
                                usage: wgpu::BufferUsages::VERTEX,
                            }
                        );
                    //green triangle
                        let vertex_points_green:Vec<Vertex> = vec![
                            Vertex::new([-0.5, -0.5]),
                            Vertex::new([0.0, 0.5]),
                            Vertex::new([0.5, -0.5]),
                        ];
                        let vertex_buffer_green = device.create_buffer_init(
                            &wgpu::util::BufferInitDescriptor {
                                label: None,
                                contents: bytemuck::cast_slice(&vertex_points_green),
                                usage: wgpu::BufferUsages::VERTEX,
                            }
                        );
                    //blue triangle
                        let vertex_points_blue:Vec<Vertex> = vec![
                            Vertex::new([-0.25, -0.5]),
                            Vertex::new([0.25, 0.5]),
                            Vertex::new([0.75, -0.5]),
                        ];
                        let vertex_buffer_blue = device.create_buffer_init(
                            &wgpu::util::BufferInitDescriptor {
                                label: None,
                                contents: bytemuck::cast_slice(&vertex_points_blue),
                                usage: wgpu::BufferUsages::VERTEX,
                            }
                        );

                    //render pass
                        {
                            let mut render_pass = encoder.begin_render_pass(
                                &wgpu::RenderPassDescriptor {
                                    label: Some("Render Pass"),
                                    color_attachments: &[
                                        wgpu::RenderPassColorAttachment {
                                            view: &multisampled_framebuffer_for_colour,
                                            resolve_target: Some(&frame_texture_view),
                                            ops: wgpu::Operations {
                                                load: wgpu::LoadOp::Clear(
                                                    wgpu::Color {
                                                        r: 1.0,
                                                        g: 1.0,
                                                        b: 1.0,
                                                        a: 1.0,
                                                    }
                                                ),
                                                store: true,
                                            },
                                        }
                                    ],
                                    depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                                        view: &multisampled_framebuffer_for_stencil,
                                        depth_ops: Some(wgpu::Operations {
                                            load: wgpu::LoadOp::Clear(0.0),
                                            store: true,
                                        }),
                                        stencil_ops: Some(wgpu::Operations {
                                            load: wgpu::LoadOp::Clear(0),
                                            store: true,
                                        }),
                                    }),
                                }
                            );

                            //stencil
                                render_pass.set_stencil_reference(0);
                                render_pass.set_vertex_buffer(0, (&vertex_buffer_stencil).slice(..));
                                render_pass.set_pipeline(&increment_write_to_stencil);
                                render_pass.draw(0..3, 0..1);
                            //red triangle
                                render_pass.set_stencil_reference(1);
                                render_pass.set_vertex_buffer(0, (&vertex_buffer_red).slice(..));
                                render_pass.set_pipeline(&regular_red);
                                render_pass.draw(0..3, 0..1);
                            //green triangle
                                render_pass.set_stencil_reference(1);
                                render_pass.set_vertex_buffer(0, (&vertex_buffer_green).slice(..));
                                render_pass.set_pipeline(&regular_green);
                                render_pass.draw(0..3, 0..1);
                            //blue triangle
                                render_pass.set_stencil_reference(1);
                                render_pass.set_vertex_buffer(0, (&vertex_buffer_blue).slice(..));
                                render_pass.set_pipeline(&regular_blue);
                                render_pass.draw(0..3, 0..1);
                        }

                    queue.submit(Some(encoder.finish()));
                    frame.present();
                }
                Event::WindowEvent {
                    event: WindowEvent::CloseRequested,
                    ..
                } => *control_flow = ControlFlow::Exit,
                _ => {}
            }
        });
}

fn main() {
    let event_loop = EventLoop::new();
    let window = winit::window::Window::new(&event_loop).unwrap();
    #[cfg(not(target_arch = "wasm32"))]
    {
        env_logger::init();
        // Temporarily avoid srgb formats for the swapchain on the web
        pollster::block_on(run(event_loop, window));
    }
    #[cfg(target_arch = "wasm32")]
    {
        std::panic::set_hook(Box::new(console_error_panic_hook::hook));
        console_log::init().expect("could not initialize logger");
        use winit::platform::web::WindowExtWebSys;
        // On wasm, append the canvas to the document body
        web_sys::window()
            .and_then(|win| win.document())
            .and_then(|doc| doc.body())
            .and_then(|body| {
                body.append_child(&web_sys::Element::from(window.canvas()))
                    .ok()
            })
            .expect("couldn't append canvas to document body");
        wasm_bindgen_futures::spawn_local(run(event_loop, window));
    }
}
