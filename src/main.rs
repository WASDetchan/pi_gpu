use std::{f64::consts::PI, num::NonZeroU64};

use wgpu::util::DeviceExt as _;

#[repr(C)]
#[derive(bytemuck::Pod, bytemuck::Zeroable, Clone, Copy)]
struct Params {
    l: f32,
    d: f32,
    seed: u64,
    n: u64,
}

fn main() {
    let n: u64 = std::env::args()
        .nth(1)
        .expect("The first argument must be repetition time")
        .parse()
        .expect("The first argument must be repetition time");

    let params = Params {
        l: 0.5,
        d: 2.0,
        n,
        seed: rand::random(),
    };

    env_logger::init();

    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());

    let adapter =
        pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))
            .expect("Failed to create adapter");

    let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
        label: None,
        required_features: wgpu::Features::SHADER_INT64
            | wgpu::Features::SHADER_INT64_ATOMIC_MIN_MAX
            | wgpu::Features::SHADER_INT64_ATOMIC_ALL_OPS,
        required_limits: wgpu::Limits::downlevel_defaults(),
        experimental_features: wgpu::ExperimentalFeatures::disabled(),
        memory_hints: wgpu::MemoryHints::MemoryUsage,
        trace: wgpu::Trace::Off,
    }))
    .expect("Failed to create device");

    let module = device.create_shader_module(wgpu::include_wgsl!("needle.wgsl"));

    let params_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: None,
        contents: bytemuck::bytes_of(&params),
        usage: wgpu::BufferUsages::UNIFORM,
    });
    let total_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: None,
        contents: bytemuck::bytes_of(&0u64),
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
    });
    let download_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: 8,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: None,
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    min_binding_size: Some(NonZeroU64::new(24).unwrap()),
                    has_dynamic_offset: false,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: false },
                    min_binding_size: Some(NonZeroU64::new(8).unwrap()),
                    has_dynamic_offset: false,
                },
                count: None,
            },
        ],
    });
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout: &bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: params_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: total_buffer.as_entire_binding(),
            },
        ],
    });
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: None,
        bind_group_layouts: &[Some(&bind_group_layout)],
        immediate_size: 0,
    });
    let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: None,
        layout: Some(&pipeline_layout),
        module: &module,
        entry_point: Some("main"),
        compilation_options: wgpu::PipelineCompilationOptions::default(),
        cache: None,
    });

    let mut encoder =
        device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

    {
        let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: None,
            timestamp_writes: None,
        });

        compute_pass.set_pipeline(&pipeline);
        compute_pass.set_bind_group(0, &bind_group, &[]);

        let workgroups = n.div_ceil(64) as u32;
        let wg_x = workgroups.min(65535);
        let wg_y = workgroups.div_ceil(65535);

        compute_pass.dispatch_workgroups(wg_x, wg_y, 1);
    }

    encoder.copy_buffer_to_buffer(&total_buffer, 0, &download_buffer, 0, total_buffer.size());

    let command_buffer = encoder.finish();

    queue.submit([command_buffer]);

    let buffer_slice = download_buffer.slice(..);
    buffer_slice.map_async(wgpu::MapMode::Read, |_| {});

    device.poll(wgpu::PollType::wait_indefinitely()).unwrap();

    let data = buffer_slice.get_mapped_range();
    let result = u64::from_le_bytes(data[..].try_into().unwrap());

    let pi = n as f64 / result as f64 * 2.0 * params.l as f64 / params.d as f64;
    let delta = (pi - PI).abs();
    let matchc = pi
        .to_string()
        .chars()
        .zip(PI.to_string().chars())
        .take_while(|(a, b)| a == b)
        .count();

    println!("{result:<11}/{n:<11}  {pi:.9}  {delta:.9}  {matchc}");
}
