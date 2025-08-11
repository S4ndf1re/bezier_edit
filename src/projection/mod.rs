use bevy::{
    asset::RenderAssetUsages,
    prelude::*,
    render::{
        render_resource::{Extent3d, TextureDimension, TextureFormat, TextureUsages},
        view::RenderLayers,
    },
};

#[derive(Component)]
#[require(Camera, Transform)]
pub struct OrthoCamera {
    image: Handle<Image>,
    /// This is the layer that the camera is actually rendering. Do not confuse this with the
    /// layer, that actually displays the texture. Everything is rendered on level 0. The texture
    /// is then placed on an object rendere in layer 1. So this camera only wants to render layer
    /// 0, while the normal camera should render both 0 and 1.
    layer: RenderLayers,
}

impl OrthoCamera {
    pub fn new(image: Handle<Image>, layer: RenderLayers) -> Self {
        Self { image, layer }
    }

    /// Receive the orthogonal Projection as an image handle
    pub fn get_image_handle(&self) -> Handle<Image> {
        self.image.clone()
    }

    pub fn get_render_layers(&self) -> RenderLayers {
        self.layer.clone()
    }
}

#[derive(Event)]
pub struct EnableOrthoCamera {
    transform: Transform,
}

impl EnableOrthoCamera {
    pub fn new(transform: Transform) -> Self {
        Self { transform }
    }
}

#[derive(Event)]
pub struct DisableOrthoCamera;

fn handle_enable_ortho_camera(
    mut commands: Commands,
    mut reader: EventReader<EnableOrthoCamera>,
    mut images: ResMut<Assets<Image>>,
    ortho_cameras: Query<Entity, With<OrthoCamera>>,
) {
    let mut transform = None;

    for evt in reader.read() {
        transform = Some(evt.transform);
    }

    if let Some(transform) = transform {
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

        let layer = RenderLayers::layer(0);

        commands.spawn((
            transform,
            OrthoCamera::new(image_handle.clone(), layer.clone()),
            Camera3d::default(),
            Camera {
                target: image_handle.clone().into(),
                clear_color: Color::BLACK.into(),
                ..default()
            },
            // NOTE: This is only for completion. This has no effect as of now, since we want to render
            // all of layer 0, but not layer 1. Layer 1 should be the orthogonal display
            layer,
        ));
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
        app.add_event::<EnableOrthoCamera>();
        app.add_event::<DisableOrthoCamera>();
    }
}
