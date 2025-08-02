use js_sys::{ArrayBuffer, Float32Array, Uint32Array};
use wgpu::util::DeviceExt; // For create_buffer_init, etc.
use wgpu::{Device, Queue};

use data_manager as data_store;

pub fn calculate_min_max_y(
    device: &Device,
    _: &Queue,
    encoder: &mut wgpu::CommandEncoder,
    data_store: &data_store::DataStore,
    mix_x: u32,
    max_x: u32,
) -> (wgpu::Buffer, wgpu::Buffer) {
    let pipelines = MinMaxPipelines::new(device);
    // let performance = web_sys::window().unwrap().performance().unwrap();
    // let start = performance.now();

    // Early return if no data is available
    if data_store.data_groups.is_empty() {
        // Return empty buffers as fallback
        let default_min_max = [0.0f32, 100.0f32];
        let staging_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Empty Staging Buffer"),
            contents: bytemuck::cast_slice(&default_min_max),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::UNIFORM,
        });
        let staging_buffer2 = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Empty Staging Buffer 2"),
            contents: bytemuck::cast_slice(&default_min_max),
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        });
        return (staging_buffer, staging_buffer2);
    }

    // Find a group with x_raw data for time range calculation
    let x_series = data_store.data_groups.iter()
        .find(|g| g.x_raw.byte_length() > 0)
        .map(|g| &g.x_raw)
        .unwrap_or(&data_store.data_groups[0].x_raw);
    
    let (start_idx, _start_val) = find_closest(mix_x, x_series);
    let start_index = start_idx;

    let (end_idx, _end_val) = find_closest(max_x, x_series);
    let end_index = end_idx + 1; // Adjust to be exclusive
    let x_data = Float32Array::new(x_series);
    let max_index = x_data.length();
    let end_index = end_index.clamp(0, max_index);

    let thread_mult = 32u32;
    let workgroup_size: u64 = 256;
    let chunk_size = workgroup_size as u32 * thread_mult;
    let sub_range_count = end_index - start_index;
    let _num_groups = sub_range_count.div_ceil(chunk_size);

    // Get y_buffers from all visible metrics across ALL data groups
    // Filter out metrics that are marked as additional_data_columns in the preset
    let mut all_y_buffers = Vec::new();

    // Get the list of metrics that should be excluded from Y bounds calculation
    // A metric should only be excluded if it ONLY appears in additional_data_columns
    // and NOT in the main data_columns
    let (_primary_metrics, additional_only_metrics) = if let Some(preset) = &data_store.preset {
        let mut primary = std::collections::HashSet::new();
        let mut additional_only = std::collections::HashSet::new();

        // First, collect all primary data columns
        for chart_type in &preset.chart_types {
            for (_, column_name) in &chart_type.data_columns {
                primary.insert(column_name.clone());
            }
        }

        // Then, collect metrics that are ONLY in additional_data_columns
        for chart_type in &preset.chart_types {
            if let Some(additional_cols) = &chart_type.additional_data_columns {
                for (_, column_name) in additional_cols {
                    // Only exclude if it's NOT also a primary metric
                    if !primary.contains(column_name) {
                        additional_only.insert(column_name.clone());
                    }
                }
            }
        }

        (primary, additional_only)
    } else {
        (
            std::collections::HashSet::new(),
            std::collections::HashSet::new(),
        )
    };

    // Process ALL data groups, not just the active one
    for (_group_idx, group) in data_store.data_groups.iter().enumerate() {
        
        for metric in &group.metrics {
            if metric.visible {
                // Check if this metric should be excluded from bounds calculation
                // Only exclude if it's ONLY an additional column and not a primary metric
                if additional_only_metrics.contains(&metric.name) {
                    continue;
                }

                all_y_buffers.extend(&metric.y_buffers);
            } else {
            }
        }
    }
    let y_buffers = &all_y_buffers;
    let num_buffers = y_buffers.len();

    // Early return if no visible buffers
    if num_buffers == 0 {
        // Return a buffer with default values
        let default_min_max = [0.0f32, 100.0f32];
        let default_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Default Min/Max Buffer"),
            contents: bytemuck::cast_slice(&default_min_max),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_SRC,
        });
        let default_staging_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Default Min/Max Staging Buffer"),
            contents: bytemuck::cast_slice(&default_min_max),
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        });
        return (default_buffer, default_staging_buffer);
    }

    // Create staging buffer large enough for all min/max pairs
    let staging_buffer_size = (2 * num_buffers * std::mem::size_of::<f32>()) as u64;
    
    // Initialize staging buffer with infinity values to ensure proper min/max computation
    // Layout: [min0, max0, min1, max1, ...]
    let mut initial_data = Vec::with_capacity(2 * num_buffers);
    for _ in 0..num_buffers {
        initial_data.push(3.402823466e+38f32);   // min (infinity)
        initial_data.push(-3.402823466e+38f32);  // max (-infinity)
    }
    
    let staging_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Staging Buffer"),
        contents: bytemuck::cast_slice(&initial_data),
        usage: wgpu::BufferUsages::COPY_DST
            | wgpu::BufferUsages::STORAGE
            | wgpu::BufferUsages::UNIFORM,
    });
    let staging_buffer2 = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Staging Buffer"),
        size: staging_buffer_size,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    

    // Need to track which y_buffer belongs to which data group
    let mut y_buffer_to_group: Vec<(usize, usize)> = Vec::new(); // (group_idx, metric_idx)
    for (group_idx, group) in data_store.data_groups.iter().enumerate() {
        for (metric_idx, metric) in group.metrics.iter().enumerate() {
            if metric.visible && !additional_only_metrics.contains(&metric.name) {
                for _ in &metric.y_buffers {
                    y_buffer_to_group.push((group_idx, metric_idx));
                }
            }
        }
    }
    

    for (buffer_index, y_buffer) in y_buffers.iter().enumerate() {
        
        // Debug: Check if buffer is mapped or has any special state
        if y_buffer.size() == 0 {
            continue;
        }
        
        // Get the data group this buffer belongs to
        let (group_idx, _metric_idx) = y_buffer_to_group[buffer_index];
        let data_group = &data_store.data_groups[group_idx];
        
        // Calculate indices for THIS specific data group
        let group_x_series = &data_group.x_raw;
        let (group_start_idx, _) = find_closest(mix_x, group_x_series);
        let (group_end_idx, _) = find_closest(max_x, group_x_series);
        let group_end_idx = group_end_idx + 1; // Exclusive
        
        let group_x_data = Float32Array::new(group_x_series);
        let group_max_index = group_x_data.length();
        let group_end_index = group_end_idx.clamp(0, group_max_index);
        let group_sub_range_count = group_end_index - group_start_idx;
        let group_num_groups = group_sub_range_count.div_ceil(chunk_size);
        

        // Create buffers for this y_buffer's first pass
        let partial_first_size = group_num_groups * 2;
        let partial_first_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Partial First Buffer"),
            size: partial_first_size as u64 * std::mem::size_of::<f32>() as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let params_first = [group_start_idx, group_end_index, chunk_size];
        
        let params_first_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("First Pass Params"),
            contents: bytemuck::cast_slice(&params_first),
            usage: wgpu::BufferUsages::UNIFORM,
        });

        // Verify buffer has STORAGE usage before binding
        if !y_buffer.usage().contains(wgpu::BufferUsages::STORAGE) {
        }
        
        // First pass bind group with current y_buffer
        let bind_group_first_pass = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("First Pass Bind Group"),
            layout: &pipelines.first_pass.get_bind_group_layout(0),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: y_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: partial_first_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: params_first_buffer.as_entire_binding(),
                },
            ],
        });

        // Record first pass
        {
            let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("First Pass Compute"),
                timestamp_writes: None,
            });
            cpass.set_pipeline(&pipelines.first_pass);
            cpass.set_bind_group(0, &bind_group_first_pass, &[]);
            cpass.dispatch_workgroups(group_num_groups, 1, 1);
        }

        // Subsequent passes
        let sub_chunk_size = 256u32;
        let mut current_in_buffer = partial_first_buffer;
        let mut current_count = group_num_groups;
        let mut pass_index = 0;

        while current_count > 1 {
            pass_index += 1;
            let next_num_groups = current_count.div_ceil(sub_chunk_size);

            let partial_out_buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(&format!("Partial Buffer Pass {pass_index}")),
                size: (next_num_groups * 2) as u64 * std::mem::size_of::<f32>() as u64,
                usage: wgpu::BufferUsages::STORAGE
                    | wgpu::BufferUsages::COPY_SRC
                    | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });

            let params_sub = [current_count, sub_chunk_size];
            let params_sub_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(&format!("Pass {pass_index} Params")),
                contents: bytemuck::cast_slice(&params_sub),
                usage: wgpu::BufferUsages::UNIFORM,
            });

            let bind_group_sub_pass = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some(&format!("Sub Pass Bind Group {pass_index}")),
                layout: &pipelines.sub_pass.get_bind_group_layout(0),
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: current_in_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: partial_out_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: params_sub_buffer.as_entire_binding(),
                    },
                ],
            });

            {
                let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some(&format!("Subsequent Pass Compute {pass_index}")),
                    timestamp_writes: None,
                });
                cpass.set_pipeline(&pipelines.sub_pass);
                cpass.set_bind_group(0, &bind_group_sub_pass, &[]);
                cpass.dispatch_workgroups(next_num_groups, 1, 1);
            }

            current_in_buffer = partial_out_buffer;
            current_count = next_num_groups;
        }

        // After all reduction passes, current_in_buffer contains just 2 floats (min, max)
        // Copy these final results to staging buffer
        let offset = (buffer_index * 2 * std::mem::size_of::<f32>()) as u64;
        encoder.copy_buffer_to_buffer(
            &current_in_buffer,
            0,  // Source offset - the final min/max are at the beginning
            &staging_buffer,
            offset,
            2 * std::mem::size_of::<f32>() as u64,  // Just copy 2 floats
        );
        encoder.copy_buffer_to_buffer(
            &current_in_buffer,
            0,
            &staging_buffer2,
            offset,
            2 * std::mem::size_of::<f32>() as u64,
        );
        
    }
    
    // Force a pipeline flush to ensure all compute passes complete
    encoder.insert_debug_marker("GPU Min/Max Calculation Complete");
    
    // Add a memory barrier to ensure all writes are visible
    encoder.push_debug_group("Ensure compute shader writes are visible");
    encoder.pop_debug_group();

    // Add a final compute pass to find overall min/max across all metrics

    let overall_shader = include_str!("overall_min_max.wgsl");
    let overall_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("Overall Min/Max Shader"),
        source: wgpu::ShaderSource::Wgsl(overall_shader.into()),
    });

    // Create buffer for overall min/max (2 floats)
    let overall_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Overall Min/Max Buffer"),
        size: 8, // 2 * f32
        usage: wgpu::BufferUsages::STORAGE
            | wgpu::BufferUsages::UNIFORM
            | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });

    // Create uniform buffer for num_metrics
    let num_metrics_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Num Metrics Buffer"),
        contents: bytemuck::cast_slice(&[num_buffers as u32]),
        usage: wgpu::BufferUsages::UNIFORM,
    });

    let overall_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("Overall Min/Max Pipeline"),
        layout: None,
        module: &overall_module,
        entry_point: Some("main"),
        cache: None,
        compilation_options: Default::default(),
    });

    let overall_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("Overall Min/Max Bind Group"),
        layout: &overall_pipeline.get_bind_group_layout(0),
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: staging_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: overall_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: num_metrics_buffer.as_entire_binding(),
            },
        ],
    });

    // Run the overall min/max computation
    {
        let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("Overall Min/Max Compute"),
            timestamp_writes: None,
        });
        cpass.set_pipeline(&overall_pipeline);
        cpass.set_bind_group(0, &overall_bind_group, &[]);
        cpass.dispatch_workgroups(1, 1, 1);
    }

    // Create a staging buffer for reading back the overall min/max
    let overall_staging_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Overall Min/Max Staging Buffer"),
        size: 8, // 2 * f32
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    // Copy overall min/max to staging buffer for CPU readback
    encoder.copy_buffer_to_buffer(
        &overall_buffer,
        0,
        &overall_staging_buffer,
        0,
        8, // 2 * f32
    );

    (overall_buffer, overall_staging_buffer)
}

fn find_closest(target: u32, data_array_buffer: &ArrayBuffer) -> (u32, u32) {
    let data = Uint32Array::new(data_array_buffer);
    let len = data.length() as usize;
    if len == 0 {
        return (0, 0);
    }
    if data.at(0).unwrap() > target {
        return (0, target);
    };
    if data.at((data.length() - 1) as i32).unwrap() < target {
        return (data.length() - 1, target);
    };
    // Perform binary search using direct JS array access
    let mut low = 0;
    let mut high = len - 1;
    let mut closest_idx = 0;
    let mut closest_diff = i32::MAX;

    while low <= high {
        let mid = (low + high) / 2;
        let val = data.get_index(mid as u32);

        match val.cmp(&target) {
            std::cmp::Ordering::Less => low = mid + 1,
            std::cmp::Ordering::Greater => high = mid - 1,
            std::cmp::Ordering::Equal => {
                closest_idx = mid;
                break;
            }
        }

        let diff = val as i32 - target as i32;
        if diff < closest_diff || (diff == closest_diff && val < target) {
            closest_diff = diff;
            closest_idx = mid;
        }
    }

    // Handle edge cases
    closest_idx = closest_idx.min(len - 1);
    let closest_val = data.get_index(closest_idx as u32);

    // Return JS array [index, value]
    (closest_idx as u32, closest_val)
}

// pub fn find_closest2(target: u32, data_array_buffer: &ArrayBuffer) -> (u32, u32) {
//     // Convert the JS ArrayBuffer to a Rust Vec<u32> for fast access
//     let data = Uint32Array::new(data_array_buffer);
//     let data_vec = data.to_vec();
//     let len = data_vec.len();

//     if len == 0 {
//         panic!("ArrayBuffer cannot be empty");
//     }

//     // Use Rust's built-in binary search for efficiency
//     match data_vec.binary_search_by(|probe| probe.partial_cmp(&target).expect("Invalid comparison"))
//     {
//         // Exact match found, return immediately
//         Ok(exact_idx) => (exact_idx as u32, data_vec[exact_idx]),
//         // Check neighboring elements for closest value
//         Err(insertion_idx) => {
//             let candidates = match insertion_idx {
//                 0 => &[0][..],
//                 l if l == len => &[len - 1][..],
//                 _ => &[insertion_idx - 1, insertion_idx][..],
//             };

//             let mut closest_idx = candidates[0];
//             let mut closest_diff = data_vec[closest_idx] - target;

//             for &candidate in candidates.iter().skip(1) {
//                 let diff = data_vec[candidate] - target;
//                 if diff < closest_diff
//                     || (diff == closest_diff && data_vec[candidate] < data_vec[closest_idx])
//                 {
//                     closest_idx = candidate;
//                     closest_diff = diff;
//                 }
//             }

//             (closest_idx as u32, data_vec[closest_idx])
//         }
//     }
// }

struct MinMaxPipelines {
    first_pass: wgpu::ComputePipeline,
    sub_pass: wgpu::ComputePipeline,
}

impl MinMaxPipelines {
    pub fn new(device: &wgpu::Device) -> Self {
        let shader_first_pass = include_str!("min_max_first.wgsl");
        let module_first_pass = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("First Pass Shader"),
            source: wgpu::ShaderSource::Wgsl(shader_first_pass.into()),
        });

        let shader_sub_pass = include_str!("min_max_second.wgsl");
        let module_sub_pass = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Sub Pass Shader"),
            source: wgpu::ShaderSource::Wgsl(shader_sub_pass.into()),
        });

        let first_pass = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("First Pass Pipeline"),
            layout: None,
            module: &module_first_pass,
            entry_point: Some("main"),
            cache: None,
            compilation_options: Default::default(),
        });

        let sub_pass = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Sub Pass Pipeline"),
            layout: None,
            module: &module_sub_pass,
            entry_point: Some("main"),
            cache: None,
            compilation_options: Default::default(),
        });

        Self {
            first_pass,
            sub_pass,
        }
    }
}
