use crate::bezier_curve_renderer::RedrawEvent;
use crate::translation_control::control_storage::ControlStorage;
use crate::util::update_material_on;
use bevy::prelude::*;
use std::f32::consts::FRAC_PI_2;

#[derive(Component)]
pub struct EnableTranslationControl;

#[derive(Component)]
struct ControlParent(Entity);
#[derive(Component)]
struct Control(Vec3);

fn register_deletes(
    mut commands: Commands,
    mut deleted: RemovedComponents<EnableTranslationControl>,
    controls: Query<(Entity, &ControlParent)>,
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
    mat_hover: Handle<StandardMaterial>,
    meshes: &mut ResMut<Assets<Mesh>>,
) {
    let cuboid = meshes.add(Cuboid::new(0.07, 0.07, 0.4));
    let line = meshes.add(Cuboid::new(0.02, 0.02, 0.8));
    let arrow = meshes.add(Cone::new(0.035, 0.2));

    child_builder
        .spawn((
            Transform::from_xyz(0.0, 0.0, -0.4),
            MeshMaterial3d(mat.clone()),
            Mesh3d(cuboid.clone()),
        ))
        .observe(update_material_on::<Pointer<Over>>(mat_hover.clone()))
        .observe(update_material_on::<Pointer<Out>>(mat.clone()));

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

fn show_transitional_controls(
    mut commands: Commands,
    to_enable: Query<(&Transform, Entity), Added<EnableTranslationControl>>,
    arrows: Res<ControlStorage>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    for (t, entity) in to_enable.iter() {
        commands
            .spawn((
                ControlParent(entity),
                Transform::from_translation(t.translation),
                Visibility::default(),
            ))
            .with_children(|parent| {
                for arrow in arrows.as_ref().iter() {
                    parent
                        .spawn((
                            Transform::from_xyz(0.0, 0.0, 0.0)
                                .looking_to(arrow.normalized, Vec3::Y),
                            Control(arrow.vec),
                            Visibility::default(),
                        ))
                        .with_children(|parent| {
                            draw_arrow(
                                parent,
                                materials.add(arrow.color),
                                materials.add(arrow.hover_color),
                                &mut meshes,
                            );
                        })
                        .observe(drag_controller)
                        .observe(drag_end_trigger_redraw);
                }
            });
    }
}

fn drag_end_trigger_redraw(
    _: Trigger<Pointer<DragEnd>>,
    mut redraw_writer: EventWriter<RedrawEvent>,
) {
    redraw_writer.send(RedrawEvent((400, 400)));
}

fn drag_controller(
    trigger: Trigger<Pointer<Drag>>,
    control_query: Query<&Control>,
    mut all_other_transforms: Query<&mut Transform, (Without<Control>, Without<ControlParent>)>,
    camera: Query<(&Camera, &GlobalTransform)>,
    parents_query: Query<&Parent>,
    mut control_parents: Query<(&ControlParent, &mut Transform)>,
    mut redraw_writer: EventWriter<RedrawEvent>,
) {
    let mut parent = None;

    for p in parents_query.iter_ancestors(trigger.entity()) {
        if control_parents.get(p).is_ok() {
            parent = Some(p);
            break;
        }
    }

    let (camera, camera_transform) = camera.single();

    let diff = {
        let mouse_start = camera
            .viewport_to_world(
                camera_transform,
                trigger.pointer_location.position - trigger.delta,
            )
            .unwrap();

        let mouse_end = camera
            .viewport_to_world(camera_transform, trigger.pointer_location.position)
            .unwrap();

        let start = mouse_start.get_point(1.0);
        let end = mouse_end.get_point(1.0);
        end - start
    };

    let axis = { control_query.get(trigger.entity()).unwrap().0.clone() };
    let direction = (diff.dot(axis)) / (diff.length() * axis.length());
    let translation = axis * direction * trigger.delta.length() * 0.01;

    let (control_parent, mut transform) = control_parents.get_mut(parent.unwrap()).unwrap();

    transform.translation = transform.translation + translation;

    let control_point = all_other_transforms.get_mut(control_parent.0);
    if control_point.is_ok() {
        let mut t = control_point.unwrap();
        t.translation = transform.translation;
    };

    redraw_writer.send(RedrawEvent((50, 50)));
}

pub struct TranslationController;

impl Plugin for TranslationController {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (show_transitional_controls));
        app.add_systems(PostUpdate, register_deletes);
    }
}
