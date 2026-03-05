use std::collections::{HashSet, hash_set::Iter};

use bevy::{
    asset::RenderAssetUsages,
    color::palettes::tailwind::{BLUE_500, GRAY_500},
    ecs::system::{SystemParam, lifetimeless::Read},
    prelude::*,
    render::{
        camera::ScalingMode,
        render_resource::{Extent3d, Face, TextureDimension, TextureFormat, TextureUsages},
        view::RenderLayers,
    },
};

use crate::{
    RootTransform,
    bezier_curve::{
        bezier_curve_renderer::EndModeEvent, components::ControlState,
        render_info::RenderInformation,
    },
    linked_entities::{
        LinkTarget, RemoveLinkedEntities, ReversePositioningType, SpawnLinkedEntities,
    },
    nurbs::parametric::Parametric,
    picking3d::picking_3d::Picking3dInteractable,
    translation_control::{
        enable_gizmo, enable_gizmo3d, translation_controller::EnableTranslationControl,
    },
};
use crate::{
    bezier_curve::bezier_curve_renderer::hover_3d,
    translation_control::translation_controller::EnableTranslationControlType,
};
use crate::{
    picking3d::events::Pointer3d, translation_control::translation_controller::CantSnapToEntities,
};

#[derive(Event)]
pub struct UpdateOrthoViews;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
/// Display an entity in either Normal camera (0), Orthogonal camera(1) or both the normal and the
/// orthogonal camera (1, 0).
pub enum DisplayIn {
    #[default]
    BothNormalAndOrtho,
    Normal,
    Ortho,
}

impl From<DisplayIn> for RenderLayers {
    fn from(value: DisplayIn) -> Self {
        match value {
            DisplayIn::BothNormalAndOrtho => RenderLayers::layer(0).with(1),
            DisplayIn::Normal => RenderLayers::layer(0),
            DisplayIn::Ortho => RenderLayers::layer(1),
        }
    }
}

#[derive(Event)]
pub struct AddBoundingEntityEvent(pub Entity);

pub fn handle_add_bounding_entity_event(
    mut reader: EventReader<AddBoundingEntityEvent>,
    mut manager: BoundingEntitiesManager,
    mut update_writer: EventWriter<UpdateOrthoViews>,
) {
    for evt in reader.read() {
        manager.add_bounding_entity(evt.0);
    }

    update_writer.write(UpdateOrthoViews);
}

#[derive(SystemParam)]
pub struct BoundingEntitiesManager<'w, 's> {
    bounding_entities: ResMut<'w, OrthoSurfaceRelevantEntities>,
    linked_spawner: SpawnLinkedEntities<'w, 's>,
    cameras: Query<'w, 's, Read<OrthoCamera>>,
    commands: Commands<'w, 's>,
    root: Query<'w, 's, Entity, With<RootTransform>>,
}

impl<'w, 's> BoundingEntitiesManager<'w, 's> {
    pub fn add_bounding_entity(&mut self, entity: Entity) {
        if !self.bounding_entities.contains(&entity) {
            if let Ok(root) = self.root.single() {
                self.commands.entity(root).with_children(|cmd| {
                    for camera in self.cameras {
                        self.linked_spawner.spawn_linked_to_entity(
                            cmd,
                            entity,
                            ReversePositioningType::OrthoProjectedOntoPlane(camera.surface_parent),
                        );
                    }
                });
            }
            self.bounding_entities.add_new_entity(entity);
        }
    }

    pub fn remove_entity(&mut self, entity: &Entity) {
        self.bounding_entities.remove_entity(entity);
    }
}

/// This list of entities determines the bounding volume of the camera perspective. Meaning,
/// that all points should be within the bounding box
#[derive(Resource, Default)]
struct OrthoSurfaceRelevantEntities {
    bounding_volume_determining_entities: HashSet<Entity>,
}

impl OrthoSurfaceRelevantEntities {
    pub fn add_new_entity(&mut self, entity: Entity) {
        self.bounding_volume_determining_entities.insert(entity);
    }

    pub fn remove_entity(&mut self, entity: &Entity) {
        self.bounding_volume_determining_entities.remove(entity);
    }

    pub fn contains(&self, entity: &Entity) -> bool {
        self.bounding_volume_determining_entities.contains(entity)
    }

    pub fn iter(&self) -> Iter<'_, Entity> {
        self.bounding_volume_determining_entities.iter()
    }
}

#[derive(Component)]
pub struct OrthoSurfaceParent {
    camera: Entity,
}

impl OrthoSurfaceParent {
    pub fn with_camera(camera: Entity) -> Self {
        Self { camera }
    }
}

#[derive(Component)]
pub struct OrthoSurfacePlane;

#[derive(Component)]
#[require(Camera, Transform)]
pub struct OrthoCamera {
    image: Handle<Image>,
    projection_size: Vec2,

    /// The surface that belongs to this camera
    surface_parent: Entity,
}

impl OrthoCamera {
    pub fn new(image: Handle<Image>, surface: Entity) -> Self {
        Self {
            image,
            projection_size: Vec2::new(1.0, 1.0),
            surface_parent: surface,
        }
    }
}

#[derive(Event)]
/// Enable an orthographic camera using global positioning.
/// The Camera is then added to the root transform, to be able to get rotated.
pub struct EnableOrthoCamera {
    transform: Transform,
}

impl EnableOrthoCamera {
    pub fn new(transform: Transform) -> Self {
        Self { transform }
    }
}

fn compute_new_transforms_for_cam_based_on_surface(surface: Transform) -> Option<Transform> {
    let mirroring_distance_seeking_ray = Ray3d::new(surface.translation, surface.forward());
    // the plane is perpendicular to th actual plane;
    let plane = InfinitePlane3d::new(surface.forward());
    let hit = mirroring_distance_seeking_ray.intersect_plane(Vec3::ZERO, plane)?;

    let mirroring_point = mirroring_distance_seeking_ray.get_point(hit);
    let dist = (surface.translation - mirroring_point).length();
    let camera_point = surface.translation + surface.forward().normalize_or_zero() * 2.0 * dist;

    Some(Transform::from_translation(camera_point).looking_at(surface.translation, -surface.up()))
}

#[allow(clippy::complexity)]
fn handle_enable_ortho_camera(
    mut commands: Commands,
    mut reader: EventReader<EnableOrthoCamera>,
    mut images: ResMut<Assets<Image>>,
    root: Query<Entity, With<RootTransform>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    transforms: Query<&Transform>,
    info: Res<RenderInformation>,
    mut linked_spawner: SpawnLinkedEntities,
    bounding_entities: Res<OrthoSurfaceRelevantEntities>,
) {
    let mut transform = None;

    for evt in reader.read() {
        transform = Some(evt.transform);
    }

    if let Some(transform) = transform {
        let size = Extent3d {
            width: 2048,
            height: 2048,
            ..default()
        };

        // This is the texture that will be rendered to.
        let mut image = Image::new_fill(
            size,
            TextureDimension::D2,
            &[0, 0, 0, 0],
            TextureFormat::Bgra8UnormSrgb,
            RenderAssetUsages::default(),
        );

        image.texture_descriptor.usage = TextureUsages::TEXTURE_BINDING
            | TextureUsages::COPY_DST
            | TextureUsages::RENDER_ATTACHMENT;

        let image_handle = images.add(image);

        let root = root.single().expect("Root must be initialized");
        let root_transform = transforms.get(root).expect("root must contain a transform");

        let affine_transform =
            root_transform.compute_affine().inverse() * transform.compute_affine();
        let transform = Transform::from_matrix(affine_transform.into());

        if let Some(cam_transform) = compute_new_transforms_for_cam_based_on_surface(transform) {
            let mut camera = None;
            let mut surface_parent = None;

            commands.entity(root).with_children(|cmd| {
                // Spawn the controlling sphere in the center of the plane
                surface_parent = Some(
                    cmd.spawn((
                        // NOTE: the OrthoSufaceParent Component will be added later, once the
                        // camera is known. This is due to simpler deletes
                        transform,
                        Mesh3d(meshes.add(Sphere::new(0.1 * info.scale))),
                        MeshMaterial3d(materials.add(StandardMaterial::from_color(BLUE_500))),
                        Picking3dInteractable::default(),
                        Name::new("Ortho Sufrace Parent"),
                        Visibility::Inherited,
                    ))
                    .with_children(|cmd| {
                        cmd.spawn((
                            OrthoSurfacePlane,
                            Transform::default(),
                            Mesh3d(
                                meshes.add(Plane3d::new(Vec3::NEG_Z, Vec2::new(10.0, 10.0) * 0.5)),
                            ),
                            MeshMaterial3d(materials.add(StandardMaterial {
                                base_color_texture: Some(image_handle.clone()),
                                cull_mode: Some(Face::Back),
                                unlit: true,
                                ..default()
                            })),
                            // This should only get rendered in layer 0
                            RenderLayers::from(DisplayIn::Normal),
                            Pickable::IGNORE,
                            Name::new("Ortho Sufrace Plane"),
                            Visibility::Inherited,
                        ));
                    })
                    .observe(handle_disable_ortho_camera)
                    .observe(handle_disable_ortho_camera3d)
                    .observe(enable_gizmo(EnableTranslationControl::new_with_root(
                        EnableTranslationControlType::WithRotation,
                    )))
                    .observe(enable_gizmo3d(EnableTranslationControl::new_with_root(
                        EnableTranslationControlType::WithRotation,
                    )))
                    .observe(hover_3d)
                    .id(),
                );

                for entity in bounding_entities.iter() {
                    if let Some(surface) = surface_parent {
                        linked_spawner.spawn_linked_to_entity(
                            cmd,
                            *entity,
                            ReversePositioningType::OrthoProjectedOntoPlane(surface),
                        );
                    }
                }

                camera = Some(
                    cmd.spawn((
                        cam_transform,
                        Name::new("Ortho Camera"),
                        OrthoCamera::new(
                            image_handle.clone(),
                            surface_parent.expect("otherwise invalid code above"),
                        ),
                        Camera3d::default(),
                        Projection::from(OrthographicProjection {
                            scaling_mode: ScalingMode::Fixed {
                                width: 10.0,
                                height: 10.0,
                            },
                            ..OrthographicProjection::default_3d()
                        }),
                        Camera {
                            // NOTE: The order is important to render first this camera, to have the
                            // image ready for the main render pass. Otherwise, flickering will
                            // appear
                            order: -1,
                            target: image_handle.clone().into(),
                            clear_color: Color::from(GRAY_500).into(),
                            ..default()
                        },
                        RenderLayers::from(DisplayIn::Ortho),
                    ))
                    .id(),
                );
            });

            if let Some(camera) = camera
                && let Some(surface_parent) = surface_parent
            {
                commands
                    .entity(surface_parent)
                    .insert(OrthoSurfaceParent::with_camera(camera));
            }
        }
    }
}

fn update_ortho_camera_positions(
    mut cameras: Query<(&OrthoCamera, &mut Transform), Without<OrthoSurfaceParent>>,
    surfaces: Query<&Transform, (With<OrthoSurfaceParent>, Without<OrthoCamera>)>,
) {
    for (camera, mut camera_transform) in &mut cameras {
        if let Ok(surface_transform) = surfaces.get(camera.surface_parent)
            && let Some(new_camera_transform) =
                compute_new_transforms_for_cam_based_on_surface(*surface_transform)
        {
            *camera_transform = new_camera_transform;
        }
    }
}

#[allow(clippy::complexity)]
/// Update the orthograpic projection planes (near, far, left, right, top, bottom) to only fit the
/// exact curve. Also, adjust the image render target to have the correct aspect ratio to not cause
/// any weird stretching. Set the target size plane.
fn update_ortho_camera_viewports(
    mut reader: EventReader<UpdateOrthoViews>,
    mut cameras: Query<(&mut OrthoCamera, &mut Projection), Without<OrthoSurfacePlane>>,
    // root: Query<&Transform, With<RootTransform>>,
    transforms: Query<&Transform, Without<RootTransform>>,
    mut images: ResMut<Assets<Image>>,
    children: Query<&Children>,
    mut surfaces: Query<
        (&mut Mesh3d, &mut MeshMaterial3d<StandardMaterial>),
        (With<OrthoSurfacePlane>, Without<OrthoCamera>),
    >,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    info: Res<RenderInformation>,
    bounding_entities: Res<OrthoSurfaceRelevantEntities>,
) {
    if reader.is_empty() {
        return;
    }
    reader.clear();

    // let root = root
    //     .single()
    //     .expect("root must be initialzed and must have a transform");

    for (mut ortho, mut projection) in &mut cameras {
        let mut max_distance = (f64::MIN, f64::MIN);

        // Only continue, if the child is actually a surface
        if let Ok(surface_transform) = transforms.get(ortho.surface_parent) {
            for entity in bounding_entities.iter() {
                if let Ok(transform) = transforms.get(*entity) {
                    let plane = super::nurbs::plane::Plane3d::from(*surface_transform);

                    if let Some((u, v)) =
                        plane.point_projected_on_plane_orthogonal(transform.translation.into())
                        && let Some((origin_x, origin_y)) = plane
                            .point_projected_on_plane_orthogonal(
                                surface_transform.translation.into(),
                            )
                    {
                        max_distance.0 = max_distance.0.max((origin_x - u).abs());
                        max_distance.1 = max_distance.1.max((origin_y - v).abs());
                    }
                }
            }
        }

        let mut max_distance = Vec2::new(max_distance.0 as f32, max_distance.1 as f32);
        if max_distance.x < 0.5 {
            max_distance.x = 0.5;
        }

        if max_distance.y < 0.5 {
            max_distance.y = 0.5;
        }

        // apply padding of 1 unit length on each side
        max_distance += Vec2::ONE * info.scale;

        // Compute the projection size and the ratio, which will then be in the interval [0, 1]
        // This is times to, in order to capture full projection
        let projection_size = 2.0 * max_distance;
        let mut ratio_x = projection_size.x;
        let mut ratio_y = projection_size.y;
        let divider = ratio_x.max(ratio_y);
        ratio_x /= divider;
        ratio_y /= divider;

        let dim_size = 2048.0;
        let size = Extent3d {
            width: (dim_size * ratio_x) as u32,
            height: (dim_size * ratio_y) as u32,
            ..default()
        };

        if let Some(image) = images.get_mut(ortho.image.id()) {
            image.resize(size);
        }
        ortho.projection_size = projection_size;

        *projection = Projection::from(OrthographicProjection {
            scaling_mode: ScalingMode::Fixed {
                width: ortho.projection_size.x,
                height: ortho.projection_size.y,
            },
            ..OrthographicProjection::default_3d()
        });

        // NOTE: since the camera only points to the parent of the surface itself, first load all
        // the children from the surfaces parent (should only be one though)
        for child in children.iter_descendants(ortho.surface_parent) {
            if let Ok((mut surface_mesh, mut material)) = surfaces.get_mut(child) {
                surface_mesh.0 = meshes.add(Plane3d::new(Vec3::NEG_Z, ortho.projection_size * 0.5));

                // NOTE: This is needed, since the MeshMaterial3d seems to not be able to
                // invalidate the RenderTarget on image changes.
                // So we have to create a complete new image. This causes flickering though, if the
                // camera order is wrong. Make sure the camera that renders to the image renders
                // before the MainCamera.
                // See https://github.com/bevyengine/bevy/issues/16159
                material.0 = materials.add(StandardMaterial {
                    base_color_texture: Some(ortho.image.clone()),
                    cull_mode: Some(Face::Back),
                    unlit: true,
                    ..default()
                });
            }
        }
    }
}

fn handle_disable_ortho_camera(
    trigger: Trigger<Pointer<Click>>,
    mut commands: Commands,
    points: Query<(Entity, &OrthoSurfaceParent)>,
    state: Res<State<ControlState>>,
    mut end_mode_writer: EventWriter<EndModeEvent>,
    mut despawner: RemoveLinkedEntities,
) {
    // Only run, when we are in the delete mode
    if *state != ControlState::Delete {
        return;
    }

    if let Ok(to_delete_parent) = points.get(trigger.target()) {
        let _ = commands
            .get_entity(to_delete_parent.1.camera)
            .map(|mut e| e.despawn());

        let _ = commands
            .get_entity(to_delete_parent.0)
            .map(|mut e| e.despawn());

        despawner.remove_link_for_all_on_same_plane(to_delete_parent.0);

        end_mode_writer.write(EndModeEvent);
    }
}

fn handle_disable_ortho_camera3d(
    trigger: Trigger<Pointer3d<crate::picking3d::events::Click>>,
    mut commands: Commands,
    points: Query<(Entity, &OrthoSurfaceParent)>,
    state: Res<State<ControlState>>,
    mut end_mode_writer: EventWriter<EndModeEvent>,
    mut despawner: RemoveLinkedEntities,
) {
    // Only run, when we are in the delete mode
    if *state != ControlState::Delete {
        return;
    }

    if let Ok(to_delete_parent) = points.get(trigger.target()) {
        let _ = commands
            .get_entity(to_delete_parent.1.camera)
            .map(|mut e| e.despawn());

        let _ = commands
            .get_entity(to_delete_parent.0)
            .map(|mut e| e.despawn());

        despawner.remove_link_for_all_on_same_plane(to_delete_parent.0);

        end_mode_writer.write(EndModeEvent);
    }
}

#[derive(SystemParam)]
/// Detect the direction(with magnitude) to the closest projected point for all cameras
pub struct ProjectedSnappingDetector<'w, 's> {
    cameras: Query<'w, 's, Read<OrthoCamera>, Without<OrthoSurfaceParent>>,
    surfaces: Query<'w, 's, Read<Transform>, (With<OrthoSurfaceParent>, Without<OrthoCamera>)>,
    transform: Query<'w, 's, Read<Transform>, (Without<OrthoCamera>, Without<OrthoSurfaceParent>)>,
    link_targets: Query<'w, 's, Read<LinkTarget>>,
    bounding_entities: Res<'w, OrthoSurfaceRelevantEntities>,
}

impl<'w, 's> ProjectedSnappingDetector<'w, 's> {
    /// Detect the closest point on any projection. Return the Directional vector that the
    /// snappable_entity must move in order to snap exactly over the projected entity
    pub fn detect_closest_projected(
        &self,
        snappable_entity: Entity,
        next_move_delta: Vec3,
        cant_snap_to_entities: &Option<CantSnapToEntities>,
    ) -> Option<Vec3> {
        let mut min_uv_distance = Vec::new();

        for camera in self.cameras {
            if let Ok(surface_transform) = self.surfaces.get(camera.surface_parent).copied()
                && let Ok(snappable_entity_transform) =
                    self.transform.get(snappable_entity).copied()
            {
                let plane = crate::nurbs::plane::Plane3d::from(surface_transform);

                if let Some(snapped_uv) = plane.point_projected_on_plane_orthogonal(
                    (snappable_entity_transform.translation + next_move_delta).into(),
                ) {
                    let snapped_uv_vec = Vec2::new(snapped_uv.0 as f32, snapped_uv.1 as f32);

                    for point in self.bounding_entities.iter() {
                        if let Some(cant_snap) = cant_snap_to_entities.clone() {
                            match cant_snap {
                                CantSnapToEntities::None => (),
                                CantSnapToEntities::All => continue,
                                CantSnapToEntities::Single(entity) => {
                                    if *point == entity {
                                        continue;
                                    }
                                }
                                CantSnapToEntities::Multiple(entities) => {
                                    if entities.contains(point) {
                                        continue;
                                    }
                                }
                            }
                        }
                        if *point != snappable_entity
                            && (self.link_targets.get(snappable_entity).is_err()
                                || self
                                    .link_targets
                                    .get(snappable_entity)
                                    .ok()
                                    .is_some_and(|e| e.parent != *point))
                            && let Ok(transform) = self.transform.get(*point)
                            && let Some(uv) = plane
                                .point_projected_on_plane_orthogonal(transform.translation.into())
                        {
                            let uv_vec = Vec2::new(uv.0 as f32, uv.1 as f32);
                            let dist = (uv_vec - snapped_uv_vec).length();
                            min_uv_distance.push((dist, snapped_uv, uv, camera.surface_parent));
                        }
                    }
                }
            }
        }

        min_uv_distance.sort_by(|a, b| a.0.total_cmp(&b.0));

        if let Some((_, snapped_uv, uv_closest, plane)) = min_uv_distance.first()
            && let Ok(surface_transform) = self.surfaces.get(*plane)
        {
            let plane = crate::nurbs::plane::Plane3d::from(*surface_transform);

            // compute the direction on the plane the the point must move, to snap to the current
            // point
            let original: Vec3 = plane.f(&[snapped_uv.0, snapped_uv.1]).into();
            let closest: Vec3 = plane.f(&[uv_closest.0, uv_closest.1]).into();
            let diff = closest - original;
            Some(diff)
        } else {
            None
        }
    }
}

pub struct ProjectionPlugin;

impl Plugin for ProjectionPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PostUpdate, (handle_enable_ortho_camera,));

        // This will run before the post update camera system update
        app.add_systems(
            PreUpdate,
            update_ortho_camera_viewports.run_if(on_event::<UpdateOrthoViews>),
        );
        app.add_systems(
            PostUpdate,
            update_ortho_camera_positions.before(handle_enable_ortho_camera),
        );
        app.add_systems(
            PostUpdate,
            handle_add_bounding_entity_event.run_if(on_event::<AddBoundingEntityEvent>),
        );
        app.add_event::<EnableOrthoCamera>();
        app.add_event::<AddBoundingEntityEvent>();
        app.add_event::<UpdateOrthoViews>();
        app.init_resource::<OrthoSurfaceRelevantEntities>();
    }
}
