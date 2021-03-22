use bevy::{
    core::FloatOrd,
    prelude::*,
    reflect::TypeUuid,
    render::{
        camera::{VisibleEntities, VisibleEntity},
        draw::{DrawContext, DrawError, Drawable},
        mesh,
        pipeline::{
            BlendDescriptor, BlendFactor, BlendOperation, ColorStateDescriptor, ColorWrite,
            CompareFunction, CullMode, DepthStencilStateDescriptor, FrontFace, PipelineDescriptor,
            PipelineSpecialization, RasterizationStateDescriptor,
            StencilStateDescriptor, StencilStateFaceDescriptor, VertexBufferDescriptor,
        },
        render_graph::{
            base::MainPass, RenderGraph,
        },
        renderer::{BindGroup, RenderResourceBindings, RenderResourceId},
        shader::{Shader, ShaderStage, ShaderStages},
        stage::DRAW,
        texture::TextureFormat,
    },
    sprite::QUAD_HANDLE,
};
use std::collections::HashMap;

pub const BATCHED_SPRITE_PIPELINE_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(PipelineDescriptor::TYPE_UUID, 13155505360388511616);

pub struct BatchingPlugin;
impl Plugin for BatchingPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_stage_after(stage::UPDATE, "update_batches", SystemStage::parallel())
            .add_system_to_stage("update_batches", update_batches.system())
            .add_stage_before(DRAW, "pre_draw", SystemStage::parallel())
            .add_system_to_stage("pre_draw", batch_system.system());

        let resources = app.resources_mut();
        let mut render_graph = resources.get_mut::<RenderGraph>().unwrap();
        render_graph.add_batched_sprite_graph(resources);
    }
}

pub struct BatchedDraw {
    is_visible: bool,
}
impl Default for BatchedDraw {
    fn default() -> Self {
        Self { is_visible: true }
    }
}

#[derive(Bundle)]
pub struct BatchedSpriteBundle {
    pub sprite: TextureAtlasSprite,
    pub atlas: Handle<TextureAtlas>,
    pub main_pass: MainPass,
    pub batched_draw: BatchedDraw,
    pub visible: Visible,

    pub transform: Transform,
    pub global_transform: GlobalTransform,
}

impl BatchedSpriteBundle {
    pub fn new(atlas: Handle<TextureAtlas>, atlas_index: u32, transform: Transform) -> Self {
        Self {
            visible: Visible {
                is_transparent: false,
                ..Default::default()
            },
            main_pass: MainPass,
            batched_draw: Default::default(),
            atlas,
            transform,
            sprite: TextureAtlasSprite {
                color: Color::default(),
                index: atlas_index,
            },
            global_transform: Default::default(),
        }
    }
}

struct SpriteBatch<'a> {
    render_resource_bindings: &'a mut RenderResourceBindings,
    atlas: &'a Handle<TextureAtlas>,
    mesh: &'a Handle<Mesh>,
    vertex_buffer_descriptor: VertexBufferDescriptor,
    instance_data: &'a [(GlobalTransform, TextureAtlasSprite)],
}

impl<'a> Drawable for SpriteBatch<'a> {
    fn draw(&mut self, draw: &mut Draw, context: &mut DrawContext) -> Result<(), DrawError> {
        context.set_pipeline(
            draw,
            &BATCHED_SPRITE_PIPELINE_HANDLE.typed(),
            &PipelineSpecialization {
                vertex_buffer_descriptor: self.vertex_buffer_descriptor.clone(),
                ..Default::default()
            },
        )?;

        let render_resource_context = &**context.render_resource_context;

        if let Some(RenderResourceId::Buffer(vertex_attribute_buffer_id)) =
            render_resource_context.get_asset_resource(self.mesh, mesh::VERTEX_ATTRIBUTE_BUFFER_ID)
        {
            draw.set_vertex_buffer(0, vertex_attribute_buffer_id, 0);
        } else {
            println!("Could not find vertex buffer for batch mesh.")
        }
        let mut indices = 0..0;
        if let Some(RenderResourceId::Buffer(quad_index_buffer)) =
            render_resource_context.get_asset_resource(self.mesh, mesh::INDEX_BUFFER_ASSET_INDEX)
        {
            draw.set_index_buffer(quad_index_buffer, 0);
            if let Some(buffer_info) = render_resource_context.get_buffer_info(quad_index_buffer) {
                indices = 0..(buffer_info.size / 4) as u32;
            } else {
                panic!("Expected buffer type.");
            }
        }

        context.set_bind_groups_from_bindings(draw, &mut [self.render_resource_bindings])?;
        context.set_asset_bind_groups(draw, self.atlas).unwrap();

        let mut transforms: Vec<_> = self
            .instance_data
            .iter()
            .map(|(t, _)| t.compute_matrix().to_cols_array())
            .collect();
        transforms.extend((0..200 - transforms.len()).map(|_| [0.0f32; 16]));
        let transforms_buffer = context.get_uniform_buffer(&transforms).unwrap();
        let mut colors: Vec<_> = self
            .instance_data
            .iter()
            .map(|(_, s)| [s.color.r(), s.color.g(), s.color.b(), s.color.a()])
            .collect();
        colors.extend((0..200 - colors.len()).map(|_| [0.0; 4]));
        let colors_buffer = context.get_uniform_buffer(&colors).unwrap();
        let mut atlas_indexes: Vec<_> = self
            .instance_data
            .iter()
            .map(|(_, s)| [s.index, 0, 0, 0])
            .collect();
        atlas_indexes.extend((0..200 - atlas_indexes.len()).map(|_| [0u32; 4]));
        assert_eq!(transforms.len(), 200);
        assert_eq!(colors.len(), 200);
        assert_eq!(atlas_indexes.len(), 200);
        let atlas_indexes_buffer = context.get_uniform_buffer(&atlas_indexes).unwrap();
        let instance_data_bind_group = BindGroup::build()
            .add_binding(0, transforms_buffer)
            .add_binding(1, colors_buffer)
            .add_binding(2, atlas_indexes_buffer)
            .finish();
        context.create_bind_group_resource(2, &instance_data_bind_group)?;
        draw.set_bind_group(2, &instance_data_bind_group);

        draw.draw_indexed(indices, 0, 0..self.instance_data.len() as u32);
        Ok(())
    }
}

pub fn build_batched_sprite_pipeline(shaders: &mut Assets<Shader>) -> PipelineDescriptor {
    PipelineDescriptor {
        rasterization_state: Some(RasterizationStateDescriptor {
            front_face: FrontFace::Ccw,
            cull_mode: CullMode::None,
            depth_bias: 0,
            depth_bias_slope_scale: 0.0,
            depth_bias_clamp: 0.0,
            clamp_depth: false,
        }),
        depth_stencil_state: Some(DepthStencilStateDescriptor {
            format: TextureFormat::Depth32Float,
            depth_write_enabled: true,
            depth_compare: CompareFunction::LessEqual,
            stencil: StencilStateDescriptor {
                front: StencilStateFaceDescriptor::IGNORE,
                back: StencilStateFaceDescriptor::IGNORE,
                read_mask: 0,
                write_mask: 0,
            },
        }),
        color_states: vec![ColorStateDescriptor {
            format: TextureFormat::default(),
            color_blend: BlendDescriptor {
                src_factor: BlendFactor::SrcAlpha,
                dst_factor: BlendFactor::OneMinusSrcAlpha,
                operation: BlendOperation::Add,
            },
            alpha_blend: BlendDescriptor {
                src_factor: BlendFactor::One,
                dst_factor: BlendFactor::One,
                operation: BlendOperation::Add,
            },
            write_mask: ColorWrite::ALL,
        }],
        ..PipelineDescriptor::new(ShaderStages {
            vertex: shaders.add(Shader::from_glsl(
                ShaderStage::Vertex,
                include_str!("sprite.vert"),
            )),
            fragment: Some(shaders.add(Shader::from_glsl(
                ShaderStage::Fragment,
                include_str!("sprite.frag"),
            ))),
        })
    }
}

pub trait BatchedSpriteRenderGraphBuilder {
    fn add_batched_sprite_graph(&mut self, resources: &Resources) -> &mut Self;
}

impl BatchedSpriteRenderGraphBuilder for RenderGraph {
    fn add_batched_sprite_graph(&mut self, resources: &Resources) -> &mut Self {
        let mut pipelines = resources.get_mut::<Assets<PipelineDescriptor>>().unwrap();
        let mut shaders = resources.get_mut::<Assets<Shader>>().unwrap();
        pipelines.set_untracked(
            BATCHED_SPRITE_PIPELINE_HANDLE,
            build_batched_sprite_pipeline(&mut shaders),
        );
        self
    }
}

struct Batch;

fn update_batches(
    commands: &mut Commands,
    mut batches: Query<Entity, With<Batch>>,
    query: Query<(&Handle<TextureAtlas>, &GlobalTransform, &BatchedDraw), With<TextureAtlasSprite>>,
) {
    let mut draws: Vec<_> = batches.iter_mut().collect();
    let mut batches = HashMap::new();
    for (atlas, t, draw) in query.iter() {
        if draw.is_visible {
            *batches
                .entry((atlas, (t.translation.z * 100.0) as i32))
                .or_insert(0) += 1;
        }
    }
    let mut count = 0;
    for c in batches.values() {
        count += c / 200 + 1;
    }

    if draws.len() < count {
        for _ in 0..count - draws.len() {
            draws.push(
                commands
                    .spawn((Draw::default(), Batch, MainPass))
                    .current_entity()
                    .unwrap(),
            );
        }
    } else if draws.len() > count {
        for e in draws.drain(0..draws.len() - count) {
            commands.despawn(e);
        }
    }
}

fn batch_system(
    mut context: DrawContext,
    mut render_resource_bindings: ResMut<RenderResourceBindings>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut batches: Query<(Entity, &mut Draw), With<Batch>>,
    mut cameras: Query<(&mut VisibleEntities, &GlobalTransform)>,
    query: Query<(
        &TextureAtlasSprite,
        &Handle<TextureAtlas>,
        &GlobalTransform,
        &BatchedDraw,
    )>,
) {
    let mut draws: Vec<_> = batches.iter_mut().collect();
    let mut batches = HashMap::new();
    for (sprite, atlas, transform, draw) in query.iter() {
        if draw.is_visible {
            batches
                .entry((atlas, 0))
                .or_insert_with(Vec::new)
                .push((*transform, sprite.clone()));
        }
    }

    if let Some((mut visible, camera_transform)) = cameras.iter_mut().next() {
        if let Some(mesh) = meshes.get_mut(QUAD_HANDLE) {
            for ((atlas, z), instance_data) in batches.into_iter() {
                for instance_data in instance_data.chunks(200) {
                    if let Some((entity, mut draw)) = draws.pop() {
                        visible.value.push(VisibleEntity {
                            entity,
                            order: FloatOrd(camera_transform.translation.z - z as f32 / 100.0),
                        });
                        let mut batch = SpriteBatch {
                            atlas,
                            render_resource_bindings: &mut render_resource_bindings,
                            mesh: &QUAD_HANDLE.typed(),
                            vertex_buffer_descriptor: mesh.get_vertex_buffer_descriptor(),
                            instance_data,
                        };

                        batch.draw(&mut draw, &mut context).unwrap();
                    }
                }
            }
        }
    }
}
