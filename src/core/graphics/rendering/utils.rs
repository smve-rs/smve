use wgpu::{CommandEncoder, LoadOp, Surface, SurfaceError, SurfaceTexture};
use crate::core::graphics::camera::components::CameraClearBehaviour;

pub fn begin_render_pass(id: &str, surface: &Surface, command_encoder: &mut CommandEncoder, clear_behaviour: &CameraClearBehaviour) -> Result<SurfaceTexture, SurfaceError> {
    let output = surface.get_current_texture()?;
    let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());
    {
        let _render_pass = command_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some(format!("Render Pass {id}").as_str()),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: match clear_behaviour {
                        CameraClearBehaviour::DontClear => {LoadOp::Load}
                        CameraClearBehaviour::Color(color) => {LoadOp::Clear(*color)}
                    },
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            occlusion_query_set: None,
            timestamp_writes: None,
        });
    }

    Ok(output)
}