use std::collections::{HashMap, HashSet};
use bevy::{
    prelude::*,
    reflect::TypeUuid,
    core::FloatOrd,
    render::{
        camera::{VisibleEntities, VisibleEntity},
        stage::DRAW,
        draw::{DrawContext, Drawable, DrawError},
        pipeline::{
            RenderPipeline, PipelineSpecialization, PipelineDescriptor,
            BlendDescriptor, BlendFactor, BlendOperation, ColorStateDescriptor, ColorWrite,
            CompareFunction, CullMode, DepthStencilStateDescriptor, FrontFace,
            RasterizationStateDescriptor, StencilStateDescriptor, StencilStateFaceDescriptor,
            VertexBufferDescriptor,
        },
        shader::{Shader, ShaderStage, ShaderStages},
        texture::TextureFormat,
        renderer::{RenderResourceId, RenderResourceBindings, BindGroup},
        render_graph::{base::self, base::MainPass, AssetRenderResourcesNode, RenderGraph, RenderResourcesNode},
        mesh,
    },
};

 pub const BATCHED_SPRITE_PIPELINE_HANDLE: HandleUntyped =
      HandleUntyped::weak_from_u64(PipelineDescriptor::TYPE_UUID, 13155505360388511616);

pub struct BatchingPlugin;
impl Plugin for BatchingPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app
            .add_stage_after(stage::UPDATE, "update_batches", SystemStage::parallel())
            .add_system_to_stage("update_batches", update_batches.system())
            .add_stage_before(DRAW, "pre_draw", SystemStage::parallel())
            .add_system_to_stage("pre_draw", inject_visibles.system())
            .add_system_to_stage("pre_draw", batch_system.system())
        ;

        let resources = app.resources_mut();
        let mut render_graph = resources.get_mut::<RenderGraph>().unwrap();
        render_graph.add_batched_sprite_graph(resources);
        let mut meshes = resources.get_mut::<Assets<Mesh>>().unwrap();
        meshes.set_untracked(
             BATCH_QUAD_HANDLE,
             Mesh::from(shape::Quad::new(Vec2::new(1.0, 1.0))),
         )
    }
}

pub struct BatchedDraw {
    is_visible: bool,
}
impl Default for BatchedDraw {
    fn default() -> Self {
        Self {
            is_visible: true
        }
    }
}

#[derive(Bundle)]
pub struct BatchedSpriteBundle {
    pub sprite: Sprite,
    pub material: Handle<ColorMaterial>,
    pub main_pass: MainPass,
    pub batched_draw: BatchedDraw,
    pub visible: Visible,

    pub transform: Transform,
    pub global_transform: GlobalTransform,
}

 pub const BATCH_QUAD_HANDLE: HandleUntyped =
      HandleUntyped::weak_from_u64(Mesh::TYPE_UUID, 14163428733613768514);

impl BatchedSpriteBundle {
    pub fn new(material: Handle<ColorMaterial>, transform: Transform) -> Self {
         Self {
             visible: Visible {
                 is_transparent: true,
                 ..Default::default()
             },
             main_pass: MainPass,
             batched_draw: Default::default(),
             sprite: Default::default(),
             material,
             transform,
             global_transform: Default::default(),
         }
    }
}

struct SpriteBatch<'a> {
    render_resource_bindings: &'a mut RenderResourceBindings,
    mesh: &'a Handle<Mesh>,
    count: usize,
    vertex_buffer_descriptor: VertexBufferDescriptor,
    transforms: &'a [GlobalTransform]
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

         if let Some(RenderResourceId::Buffer(vertex_attribute_buffer_id)) = render_resource_context
             .get_asset_resource(
                 self.mesh,
                 mesh::VERTEX_ATTRIBUTE_BUFFER_ID,
             )
         {
             println!("setting buffer: {:?}", vertex_attribute_buffer_id);
             draw.set_vertex_buffer(0, vertex_attribute_buffer_id, 0);
         } else {
             println!("Could not find vertex buffer for batch mesh.")
         }
         let mut indices = 0..0;
         if let Some(RenderResourceId::Buffer(quad_index_buffer)) = render_resource_context
             .get_asset_resource(
                 self.mesh,
                 mesh::INDEX_BUFFER_ASSET_INDEX,
             )
         {
             draw.set_index_buffer(quad_index_buffer, 0);
             if let Some(buffer_info) = render_resource_context.get_buffer_info(quad_index_buffer) {
                 indices = 0..(buffer_info.size / 4) as u32;
             } else {
                 panic!("Expected buffer type.");
             }
         }

         context.set_bind_groups_from_bindings(draw, &mut [self.render_resource_bindings])?;

        let transforms:Vec<_> = self.transforms.iter().map(|t| t.compute_matrix().to_cols_array()).collect();
        let transforms_buffer = context.get_uniform_buffer(&transforms).unwrap();
         let transforms_bind_group = BindGroup::build()
            .add_binding(0, transforms_buffer)
            .finish();
        context.create_bind_group_resource(3, &transforms_bind_group)?;
        draw.set_bind_group(3, &transforms_bind_group);


         draw.draw_indexed(indices.clone(), 0, 0..self.count as u32);
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

pub mod node {
     pub const COLOR_MATERIAL: &str = "color_material";
     pub const SPRITE: &str = "sprite";
 }

 pub trait BatchedSpriteRenderGraphBuilder {
     fn add_batched_sprite_graph(&mut self, resources: &Resources) -> &mut Self;
 }

 impl BatchedSpriteRenderGraphBuilder for RenderGraph {
     fn add_batched_sprite_graph(&mut self, resources: &Resources) -> &mut Self {
         self.add_system_node(
             node::COLOR_MATERIAL,
             AssetRenderResourcesNode::<ColorMaterial>::new(false),
         );
         self.add_node_edge(node::COLOR_MATERIAL, base::node::MAIN_PASS)
             .unwrap();

         self.add_system_node(node::SPRITE, RenderResourcesNode::<Sprite>::new(true));
         self.add_node_edge(node::SPRITE, base::node::MAIN_PASS)
             .unwrap();

         let mut pipelines = resources.get_mut::<Assets<PipelineDescriptor>>().unwrap();
         let mut shaders = resources.get_mut::<Assets<Shader>>().unwrap();
         pipelines.set_untracked(BATCHED_SPRITE_PIPELINE_HANDLE, build_batched_sprite_pipeline(&mut shaders));
         self
     }
 }

struct Batch;

fn update_batches(
    commands: &mut Commands,
    mut batches: Query<Entity, With<Batch>>,
    query: Query<(&Sprite, &Handle<ColorMaterial>, &BatchedDraw)>,
) {
    let mut draws:Vec<_> = batches.iter_mut().collect();
    let mut batches = HashSet::new();
    for (sprite, material, draw) in query.iter() {
        if draw.is_visible {
            batches.insert(((sprite.size.x as i32, sprite.size.y as i32), material));
        }
    }
    if draws.len() < batches.len() {
        for _ in 0..batches.len()-draws.len() {
            draws.push(commands.spawn((Draw::default(), Batch, RenderPipelines::from_pipelines(vec![RenderPipeline::new(BATCHED_SPRITE_PIPELINE_HANDLE.typed(),)]), MainPass)).current_entity().unwrap());
        }
    } else if draws.len() > batches.len() {
        for e in draws.drain(0..draws.len()-batches.len()) {
            commands.despawn(e);
        }
    }
    draws.sort();
    let mut batches:Vec<_> = batches.into_iter().collect();
    batches.sort();
    for (e, (sprite, material)) in draws.into_iter().zip(batches.into_iter()) {
        let sprite:Sprite = Sprite {
            size: Vec2::new(sprite.0 as f32, sprite.1 as f32),
            ..Default::default()
        };
        let material: Handle<ColorMaterial> = material.clone_weak();
        commands.insert(e, (sprite, material));
    }

}

fn inject_visibles(
    mut cameras: Query<&mut VisibleEntities>,
    batches: Query<Entity, With<Batch>>,
) {
    if let Some(mut visible) = cameras.iter_mut().next() {
        for entity in batches.iter() {
            visible.value.push(VisibleEntity { entity, order: FloatOrd(0.0) });
        }
    }
}

fn batch_system(
    mut context: DrawContext,
    mut render_resource_bindings: ResMut<RenderResourceBindings>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut batches: Query<(Entity, &mut Draw), With<Batch>>,
    query: Query<(&Sprite, &Handle<ColorMaterial>, &GlobalTransform, &BatchedDraw)>,
) {
    println!("{:#?}", *render_resource_bindings);
    let mut draws:Vec<_> = batches.iter_mut().collect();
    let mut batches = HashMap::new();
    for (sprite, material, transform, draw) in query.iter() {
        if draw.is_visible {
            batches.entry(((sprite.size.x as i32, sprite.size.y as i32), material)).or_insert_with(|| vec![]).push(*transform);
        }
    }
    println!("{:?}", draws.iter().map(|(e, _)| e));
    draws.sort_by_key(|(e, _)| *e);
    let mut batches:Vec<_> = batches.into_iter().collect();
    batches.sort_by_key(|(k, _)| *k);

    if let Some(mesh) = meshes.get_mut(BATCH_QUAD_HANDLE) {
        for ((_key, transforms), (_e, mut draw)) in batches.into_iter().zip(draws.into_iter()) {
            let mut batch = SpriteBatch {
                render_resource_bindings: &mut render_resource_bindings,
                mesh: &BATCH_QUAD_HANDLE.typed(),
                count: transforms.len(),
                vertex_buffer_descriptor: mesh.get_vertex_buffer_descriptor(),
                transforms: &transforms,
            };
            println!("batch size: {}", batch.count);

            batch.draw(&mut draw, &mut context).unwrap();
        }
    }
}
