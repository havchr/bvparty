use image::GenericImageView;
use anyhow::*;
use wgpu::Device;

pub struct Texture {
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub sampler: wgpu::Sampler,
}

impl Texture {
    pub fn from_bytes(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        bytes: &[u8],
        label: &str,
    ) -> Result<Self> {
        let img = image::load_from_memory(bytes)?;
        Self::from_image(device, queue, &img, Some(label))
    }


    pub fn from_image(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        img: &image::DynamicImage,
        label: Option<&str>,
    ) -> Result<Self> {
        let rgba = img.to_rgba8();
        let dimensions = img.dimensions();
        let texture_size = wgpu::Extent3d {
            width: dimensions.0,
            height: dimensions.1,
            depth_or_array_layers: 1,
        };

        let texture = device.create_texture(
            &wgpu::TextureDescriptor {
                label,
                size: texture_size,
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8Unorm,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            }
        );

        queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &rgba,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(4 * dimensions.0),
                rows_per_image: Some(dimensions.1),
            },
            texture_size,
        );

        let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        Ok(Self { texture, sampler, view: texture_view})
    }

    pub fn create_rtt_texture(width: u32, height: u32, device: &wgpu::Device,format : wgpu::TextureFormat, label: Option<&str>)
    -> Result<Self>
    {
        let texture_descriptor_rtt = wgpu::TextureDescriptor {
            label,
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        };

        let texture_rtt = device.create_texture(&texture_descriptor_rtt);

        let sampler_rtt = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });
        let texture_view = texture_rtt.create_view(&wgpu::TextureViewDescriptor::default());
        Ok(Self { texture:texture_rtt, sampler:sampler_rtt, view: texture_view})
    }

    pub const DEPTH_FORMAT : wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;
    pub fn create_depth_texture(device: &wgpu::Device,config: &wgpu::SurfaceConfiguration, label: &str)
        -> Self
    {
        let size = wgpu::Extent3d{
            width: config.width,
            height: config.height,
            depth_or_array_layers:1
        };

        let desc = wgpu::TextureDescriptor{
         label: Some(label)   ,
            size,
            mip_level_count : 1,
            sample_count : 1,
            dimension: wgpu::TextureDimension::D2,
            format: Self::DEPTH_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
            | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[]
        };

        let texture = device.create_texture(&desc);
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(
            &wgpu::SamplerDescriptor{

                address_mode_u: wgpu::AddressMode::ClampToEdge,
                address_mode_v: wgpu::AddressMode::ClampToEdge,
                address_mode_w: wgpu::AddressMode::ClampToEdge,
                mag_filter: wgpu::FilterMode::Linear,
                min_filter: wgpu::FilterMode::Linear,
                mipmap_filter: wgpu::FilterMode::Nearest,
                compare: Some(wgpu::CompareFunction::LessEqual),
                lod_min_clamp: 0.0,
                lod_max_clamp: 100.0,
                ..Default::default()
            }
        );

        Self{texture,view,sampler}
    }

}

/*
so, bind group and bind group layout seems to describe
"multi-texturing" in old school terms.
I am guessing this must match with the shader as well,i.e if
you say 3 textures, and your shader reads 4, mayhem!?!?
 */
pub fn setup_texture_stage(device: &Device, textures: &[&Texture], label: Option<&str> )
                           -> Result<(wgpu::BindGroupLayout,wgpu::BindGroup)> {

    let mut bind_group_entries: Vec<wgpu::BindGroupEntry> = Vec::new();
    let mut bind_group_layout_entries: Vec<wgpu::BindGroupLayoutEntry> = Vec::new();
    for i in 0..textures.len() {

       let layout_entry_1 = wgpu::BindGroupLayoutEntry {
        binding: i as u32,
           visibility: wgpu::ShaderStages::FRAGMENT,
           ty: wgpu::BindingType::Texture {
               multisampled: false,
               view_dimension: wgpu::TextureViewDimension::D2,
               sample_type: wgpu::TextureSampleType::Float {
                   filterable: true
               },
           },
           count: None,
       };

        let layout_entry_2 = wgpu::BindGroupLayoutEntry {
            binding: (i+1) as u32,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
            count: None,
        };

        let entry_1  = wgpu::BindGroupEntry {
            binding: i as u32,
            resource: wgpu::BindingResource::TextureView(&textures[i].view),
        };
        let entry_2  = wgpu::BindGroupEntry {
            binding: (i+1) as u32,
            resource: wgpu::BindingResource::Sampler(&textures[i].sampler),
        };
        bind_group_entries.push(entry_1);
        bind_group_entries.push(entry_2);
        bind_group_layout_entries.push(layout_entry_1);
        bind_group_layout_entries.push(layout_entry_2);
    }


    let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        entries: &bind_group_layout_entries.as_slice(),
        label,
    });

    let bind_group = device.create_bind_group(
        &wgpu::BindGroupDescriptor {
            layout: &bind_group_layout,
            entries: &bind_group_entries.as_slice(),
            label,
        }
    );

    Ok((bind_group_layout,bind_group))

}


