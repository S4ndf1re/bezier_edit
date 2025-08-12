use bevy::{
    asset::RenderAssetUsages,
    prelude::*,
    render::{
        render_resource::{Extent3d, TextureDimension, TextureFormat, TextureUsages},
        view::RenderLayers,
    },
};

use crate::RootTransform;

#[derive(Component)]
pub struct OrthoSurface;

#[derive(Component)]
#[require(Camera, Transform)]
pub struct OrthoCamera {
    /// This list of entities determines the bounding volume of the camera perspective. Meaning,
    /// that all points should be within the bounding box
    bounding_volume_determining_entities: Vec<Entity>,
    image: Handle<Image>,
    projection_size: Vec2,

    /// The surface that belongs to this camera
    surface: Entity,
}

impl OrthoCamera {
    pub fn new(image: Handle<Image>, bounding_entities: Vec<Entity>, surface: Entity) -> Self {
        Self {
            image,
            bounding_volume_determining_entities: bounding_entities,
            projection_size: Vec2::new(1.0, 1.0),
            surface,
        }
    }
}

#[derive(Event)]
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

#[derive(Event)]
pub struct DisableOrthoCamera;

fn compute_new_transforms_for_cam_based_on_surface(surface: Transform) -> Option<Transform> {
    let mirroring_distance_seeking_ray = Ray3d::new(surface.translation, surface.forward());
    // the plane is perpendicular to th actual plane;
    let plane = InfinitePlane3d::new(surface.forward());
    let hit = mirroring_distance_seeking_ray.intersect_plane(Vec3::ZERO, plane)?;

    let mirroring_point = mirroring_distance_seeking_ray.get_point(hit);

    Some(Transform::from_translation(mirroring_point).looking_to(-surface.forward(), Vec3::Y))
}

fn handle_enable_ortho_camera(
    mut commands: Commands,
    mut reader: EventReader<EnableOrthoCamera>,
    mut images: ResMut<Assets<Image>>,
    ortho_cameras: Query<Entity, With<OrthoCamera>>,
    root: Query<Entity, With<RootTransform>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
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
        for entity in ortho_cameras {
            commands.entity(entity).despawn();
        }

        let size = Extent3d {
            width: 512,
            height: 512,
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

        if let Some(cam_transform) = compute_new_transforms_for_cam_based_on_surface(transform) {
            let root = root.single().unwrap();
            commands.entity(root).with_children(|cmd| {
                let surface = cmd
                    .spawn((
                        OrthoSurface,
                        transform,
                        Mesh3d(meshes.add(Cuboid::new(1.0, 1.0, 0.0001))),
                        MeshMaterial3d(materials.add(StandardMaterial {
                            base_color_texture: Some(image_handle.clone()),
                            ..default()
                        })),
                        // This should only get rendered in layer 1
                        RenderLayers::layer(1),
                    ))
                    .id();

                cmd.spawn((
                    cam_transform,
                    OrthoCamera::new(image_handle.clone(), bounding_entities, surface),
                    Camera3d::default(),
                    Projection::from(OrthographicProjection::default_3d()),
                    Camera {
                        target: image_handle.clone().into(),
                        clear_color: Color::BLACK.into(),
                        ..default()
                    },
                ));
            });
        }
    }
}

fn update_ortho_camera_positions(
    mut cameras: Query<(&OrthoCamera, &mut Transform), Without<OrthoSurface>>,
    surfaces: Query<&Transform, (With<OrthoSurface>, Without<OrthoCamera>)>,
) {
    for (camera, mut camera_transform) in &mut cameras {
        if let Ok(surface_transform) = surfaces.get(camera.surface)
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
    mut cameras: Query<
        (
            &mut Camera,
            &mut OrthoCamera,
            &GlobalTransform,
            &mut Projection,
        ),
        Without<OrthoSurface>,
    >,
    transforms: Query<&GlobalTransform>,
    mut images: ResMut<Assets<Image>>,
    mut surfaces: Query<
        (&mut Mesh3d, &mut MeshMaterial3d<StandardMaterial>),
        (With<OrthoSurface>, Without<OrthoCamera>),
    >,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for (mut camera, mut ortho, camera_transform, mut projection) in &mut cameras {
        let mut min_corner = Vec2::new(f32::MAX, f32::MAX);
        let mut max_corner = Vec2::new(f32::MIN, f32::MIN);
        let mut far = f32::MIN;
        let mut near = f32::MAX;

        assert!(!ortho.bounding_volume_determining_entities.is_empty());

        for entity in &ortho.bounding_volume_determining_entities {
            if let Ok(transform) = transforms.get(*entity)
                && let Ok(view) =
                    camera.world_to_viewport_with_depth(camera_transform, transform.translation())
            {
                min_corner.x = min_corner.x.min(view.x);
                min_corner.y = min_corner.x.min(view.y);

                max_corner.x = max_corner.x.max(view.x);
                max_corner.y = max_corner.x.max(view.y);

                far = far.max(view.z);
                near = near.min(view.z);
            }
        }

        let area = Rect::from_corners(min_corner, max_corner);
        *projection = Projection::from(OrthographicProjection {
            area,
            near,
            far,
            ..OrthographicProjection::default_3d()
        });

        // Compute the projection size and the ratio, which will then be in the interval [0, 1]
        let projection_size = area.max - area.min;
        let mut ratio_x = projection_size.x;
        let mut ratio_y = projection_size.y;
        let divider = ratio_y.max(ratio_y);
        ratio_x /= divider;
        ratio_y /= divider;

        let dim_size = 512.0;
        let size = Extent3d {
            width: (dim_size * ratio_x) as u32,
            height: (dim_size * ratio_y) as u32,
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
        camera.target = image_handle.clone().into();
        ortho.image = image_handle.clone();
        ortho.projection_size = projection_size;

        if let Ok((mut surface_mesh, mut surface_mat)) = surfaces.get_mut(ortho.surface) {
            surface_mesh.0 = meshes.add(Cuboid::new(
                ortho.projection_size.x,
                ortho.projection_size.y,
                0.0001,
            ));

            surface_mat.0 = materials.add(StandardMaterial {
                base_color_texture: Some(image_handle.clone()),
                ..default()
            })
        }
    }
}

fn handle_disable_ortho_camera(
    mut reader: EventReader<DisableOrthoCamera>,
    mut commands: Commands,
    cameras: Query<Entity, With<OrthoCamera>>,
) {
    if reader.is_empty() {
        return;
    }

    reader.clear();

    for cam in cameras {
        commands.entity(cam).despawn();
    }
}

pub struct ProjectionPlugin;

impl Plugin for ProjectionPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            PostUpdate,
            (
                handle_enable_ortho_camera,
                handle_disable_ortho_camera.before(handle_enable_ortho_camera),
            ),
        );

        // This will run before the post update camera system update
        app.add_systems(PreUpdate, update_ortho_camera_viewports);
        app.add_systems(Update, update_ortho_camera_positions);
        app.add_event::<EnableOrthoCamera>();
        app.add_event::<DisableOrthoCamera>();
    }
}
