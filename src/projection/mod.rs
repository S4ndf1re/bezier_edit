use bevy::{
    asset::RenderAssetUsages,
    color::palettes::tailwind::{BLUE_500, GREEN_800, PINK_700, RED_600},
    prelude::*,
    render::{
        camera::{CameraProjection, ScalingMode},
        render_resource::{Extent3d, Face, TextureDimension, TextureFormat, TextureUsages},
        view::RenderLayers,
    },
};

use crate::{
    RootTransform,
    bezier_curve::{
        bezier_curve_renderer::{EndModeEvent, RedrawEvent},
        components::ControlState,
        render_info::RenderInformation,
    },
    picking3d::picking_3d::Picking3dInteractable,
    translation_control::enable_gizmo,
};

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
    /// This list of entities determines the bounding volume of the camera perspective. Meaning,
    /// that all points should be within the bounding box
    bounding_volume_determining_entities: Vec<Entity>,
    image: Handle<Image>,
    projection_size: Vec2,

    /// The surface that belongs to this camera
    surface_parent: Entity,
}

impl OrthoCamera {
    pub fn new(image: Handle<Image>, bounding_entities: Vec<Entity>, surface: Entity) -> Self {
        Self {
            image,
            bounding_volume_determining_entities: bounding_entities,
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
    bounding_entities: Vec<Entity>,
}

impl EnableOrthoCamera {
    pub fn new(transform: Transform, bounding_entities: Vec<Entity>) -> Self {
        Self {
            transform,
            bounding_entities,
        }
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

    Some(Transform::from_translation(camera_point).looking_at(surface.translation, surface.up()))
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
) {
    let mut transform = None;
    let mut bounding_entities = None;

    for evt in reader.read() {
        transform = Some(evt.transform);
        bounding_entities = Some(evt.bounding_entities.clone());
    }

    if let Some(transform) = transform
        && let Some(bounding_entities) = bounding_entities
    {
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

        let root = root.single().unwrap();
        let root_transform = transforms.get(root).unwrap();
        let affine = transform.compute_affine() * root_transform.compute_affine().inverse();

        let transform = Transform::from_matrix(affine.into());

        if let Some(cam_transform) = compute_new_transforms_for_cam_based_on_surface(transform) {
            info!("Transform: {:?}", transform);
            info!("Camera Transform: {:?}", cam_transform);
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
                        Picking3dInteractable,
                    ))
                    .with_children(|cmd| {
                        cmd.spawn((
                            OrthoSurfacePlane,
                            Transform::default(),
                            Mesh3d(meshes.add(Cuboid::new(10.0, 10.0, 0.0001))),
                            MeshMaterial3d(materials.add(StandardMaterial {
                                base_color_texture: Some(image_handle.clone()),
                                cull_mode: Some(Face::Back),
                                unlit: true,
                                ..default()
                            })),
                            // This should only get rendered in layer 0
                            RenderLayers::from(DisplayIn::Normal),
                            Pickable::IGNORE,
                        ));
                    })
                    .observe(handle_disable_ortho_camera)
                    .observe(enable_gizmo::<true>)
                    .id(),
                );

                camera = Some(
                    cmd.spawn((
                        cam_transform,
                        OrthoCamera::new(
                            image_handle.clone(),
                            bounding_entities,
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
                            target: image_handle.clone().into(),
                            clear_color: Color::WHITE.into(),
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
    mut cameras: Query<(&OrthoCamera, &mut Transform), Without<OrthoSurfacePlane>>,
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
    mut reader: EventReader<RedrawEvent>,
    mut cameras: Query<
        (&mut OrthoCamera, &GlobalTransform, &mut Projection),
        Without<OrthoSurfacePlane>,
    >,
    transforms: Query<&GlobalTransform>,
    mut images: ResMut<Assets<Image>>,
    children: Query<&Children>,
    mut surfaces: Query<
        (&mut Mesh3d, &mut MeshMaterial3d<StandardMaterial>),
        (With<OrthoSurfacePlane>, Without<OrthoCamera>),
    >,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    if reader.is_empty() {
        return;
    }
    reader.clear();

    for (mut ortho, camera_transform, mut projection) in &mut cameras {
        let mut min_corner = Vec2::new(f32::MAX, f32::MAX);
        let mut max_corner = Vec2::new(f32::MIN, f32::MIN);
        let mut far = f32::MIN;
        let mut near = f32::MAX;

        assert!(!ortho.bounding_volume_determining_entities.is_empty());

        for entity in &ortho.bounding_volume_determining_entities {
            if let Ok(transform) = transforms.get(*entity) {
                for child in children.iter_descendants(ortho.surface_parent) {
                    if let Ok(surface_transform) = transforms.get(child) {
                        let plane = InfinitePlane3d::new(surface_transform.forward());
                        let ray = Ray3d::new(transform.translation(), camera_transform.forward());

                        // TODO: This is still buggy has hell
                        if let Some(hit) =
                            ray.intersect_plane(surface_transform.translation(), plane)
                        {
                            let point = ray.get_point(hit);
                            let up = surface_transform.up().normalize_or_zero();
                            let left = surface_transform.left().normalize_or_zero();

                            let delta_p = surface_transform.translation() - point;

                            let (u, v) = (delta_p.dot(left), delta_p.dot(up));
                            min_corner.x = min_corner.x.min(u);
                            min_corner.y = min_corner.y.min(v);

                            max_corner.x = max_corner.x.max(u);
                            max_corner.y = max_corner.y.max(v);
                            far = far.max(hit);
                            near = near.min(hit);
                        }
                    }
                }
            }
        }

        if max_corner.x - min_corner.x < 0.5 || max_corner.x - min_corner.x > 1000.0 {
            max_corner.x = 0.25;
            min_corner.x = -0.25;
        }

        if max_corner.y - min_corner.y < 0.5 || max_corner.y - min_corner.y > 1000.0 {
            max_corner.y = 0.25;
            min_corner.y = -0.25;
        }

        info!("min corner {min_corner}");
        info!("max corner {max_corner}");

        // apply padding of 1 unit length on each side
        min_corner += Vec2::new(-1.0, -1.0);
        max_corner += Vec2::new(1.0, 1.0);

        // info!("corners: {min_corner:?}, {max_corner:?}");
        let area = Rect::from_corners(min_corner, max_corner);

        // Compute the projection size and the ratio, which will then be in the interval [0, 1]
        let projection_size = area.max - area.min;
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
                surface_mesh.0 = meshes.add(Cuboid::new(
                    ortho.projection_size.x,
                    ortho.projection_size.y,
                    0.0001,
                ));

                // NOTE: This is needed, since the MeshMaterial3d seems to not be able to
                // invalidate the RenderTarget on image changes.
                // So we have to create a complete new image. This causes flickering though.
                // See https://github.com/bevyengine/bevy/issues/16159
                materials.set_changed();
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
) {
    // Only run, when we are in the delete mode
    if *state != ControlState::Delete {
        return;
    }

    if let Ok(to_delete_parent) = points.get(trigger.target()) {
        if let Ok(mut entity) = commands.get_entity(to_delete_parent.1.camera) {
            entity.despawn();
        }

        if let Ok(mut entity) = commands.get_entity(to_delete_parent.0) {
            entity.despawn();
        }

        end_mode_writer.write(EndModeEvent);
    }
}

pub struct ProjectionPlugin;

impl Plugin for ProjectionPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PostUpdate, (handle_enable_ortho_camera,));

        // This will run before the post update camera system update
        app.add_systems(PreUpdate, update_ortho_camera_viewports);
        app.add_systems(
            PostUpdate,
            update_ortho_camera_positions.before(handle_enable_ortho_camera),
        );
        app.add_event::<EnableOrthoCamera>();
    }
}
