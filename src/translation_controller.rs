use bevy::color::palettes::tailwind::*;
use bevy::prelude::*;
use std::f32::consts::FRAC_PI_2;

#[derive(Component)]
pub struct EnableTranslationControl;

#[derive(Component)]
struct Control(Entity, Vec3);

fn register_deletes(
    mut commands: Commands,
    mut deleted: RemovedComponents<EnableTranslationControl>,
    controls: Query<(Entity, &Control)>,
) {
    for event in deleted.read() {
        for (entity, contrl) in controls.iter() {
            if contrl.0 == event {
                commands.entity(entity).despawn_recursive();
            }
        }
    }
}

fn draw_arrow(
    child_builder: &mut ChildBuilder,
    mat: Handle<StandardMaterial>,
    mut meshes: &mut ResMut<Assets<Mesh>>,
) {
    let cuboid = meshes.add(Cuboid::new(0.07, 0.07, 0.4));
    let line = meshes.add(Cuboid::new(0.02, 0.02, 0.8));
    let arrow = meshes.add(Cone::new(0.035, 0.2));

    child_builder.spawn((
        Transform::from_xyz(0.0, 0.0, -0.4),
        MeshMaterial3d(mat.clone()),
        Mesh3d(cuboid.clone()),
    ));

    child_builder.spawn((
        Transform::from_xyz(0.0, 0.0, -0.4),
        MeshMaterial3d(mat.clone()),
        Mesh3d(line.clone()),
    ));

    let mut transform = Transform::from_xyz(0.0, 0.0, -0.9);
    transform.rotate_x(-FRAC_PI_2);
    child_builder.spawn((
        transform,
        MeshMaterial3d(mat.clone()),
        Mesh3d(arrow.clone()),
    ));
}

fn render_helper_gizmos(
    to_enable: Query<(&Transform, Entity), With<EnableTranslationControl>>,
    mut gizmos: Gizmos,
) {
    for (t, _) in to_enable.iter() {
        gizmos.arrow(
            t.translation,
            t.translation + Vec3::X * 100.0,
            Color::from(RED_600),
        );
        gizmos.arrow(
            t.translation,
            t.translation + Vec3::Y * 100.0,
            Color::from(GREEN_600),
        );
        gizmos.arrow(
            t.translation,
            t.translation + Vec3::Z * 100.0,
            Color::from(BLUE_600),
        );
    }
}

fn show_transitional_controls(
    mut commands: Commands,
    to_enable: Query<(&Transform, Entity), Added<EnableTranslationControl>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    let blue = materials.add(Color::from(BLUE_600));
    let red = materials.add(Color::from(RED_600));
    let green = materials.add(Color::from(GREEN_600));

    for (t, entity) in to_enable.iter() {
        commands
            .spawn((
                Transform::from_translation(t.translation).looking_to(Vec3::X, Vec3::Y),
                Control(entity, Vec3::X),
                Visibility::default(),
            ))
            .with_children(|parent| {
                draw_arrow(parent, red.clone(), &mut meshes);
            });

        commands
            .spawn((
                Transform::from_translation(t.translation).looking_to(Vec3::Y, Vec3::Y),
                Control(entity, Vec3::Y),
                Visibility::default(),
            ))
            .with_children(|parent| {
                draw_arrow(parent, green.clone(), &mut meshes);
            });

        commands
            .spawn((
                Transform::from_translation(t.translation).looking_to(Vec3::Z, Vec3::Y),
                Control(entity, Vec3::Z),
                Visibility::default(),
            ))
            .with_children(|parent| {
                draw_arrow(parent, blue.clone(), &mut meshes);
            });
    }
}

pub struct TranslationController;

impl Plugin for TranslationController {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (show_transitional_controls));
        app.add_systems(PostUpdate, register_deletes);
    }
}
