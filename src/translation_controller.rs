use crate::bezier_curve_renderer::{RedrawEvent, ScaleInformation};
use crate::picking_3d;
use crate::picking_3d::{MoveIn, MoveOut, Pointer3d};
use crate::util::update_material_on;
use bevy::color::palettes::tailwind::*;
use bevy::ecs::relationship::RelatedSpawnerCommands;
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
    child_builder: &mut RelatedSpawnerCommands<ChildOf>,
    mat: Handle<StandardMaterial>,
    mat_hover: Handle<StandardMaterial>,
    meshes: &mut ResMut<Assets<Mesh>>,
    scale: f32,
) {
    let cuboid = meshes.add(Cuboid::new(0.07 * scale, 0.07 * scale, 0.4 * scale));
    let line = meshes.add(Cuboid::new(0.02 * scale, 0.02 * scale, 0.8 * scale));
    let arrow = meshes.add(Cone::new(0.035 * scale, 0.2 * scale));

    child_builder
        .spawn((
            Transform::from_xyz(0.0, 0.0, -0.4 * scale),
            MeshMaterial3d(mat.clone()),
            Mesh3d(cuboid.clone()),
        ))
        .observe(update_material_on::<Pointer<Over>>(mat_hover.clone()))
        .observe(update_material_on::<Pointer<Out>>(mat.clone()))
        .observe(update_material_on::<Pointer3d<MoveIn>>(mat_hover.clone()))
        .observe(update_material_on::<Pointer3d<MoveOut>>(mat.clone()));

    child_builder.spawn((
        Transform::from_xyz(0.0, 0.0, -0.4 * scale),
        MeshMaterial3d(mat.clone()),
        Mesh3d(line.clone()),
    ));

    let mut transform = Transform::from_xyz(0.0, 0.0, -0.9 * scale);
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
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
    scale: Res<ScaleInformation>,
) {
    let scale = scale.scale;
    let blue = materials.add(Color::from(BLUE_600));
    let blue_hover = materials.add(Color::from(BLUE_800));
    let red = materials.add(Color::from(RED_600));
    let red_hover = materials.add(Color::from(RED_800));
    let green = materials.add(Color::from(GREEN_600));
    let green_hover = materials.add(Color::from(GREEN_800));

    for (t, entity) in to_enable.iter() {
        commands
            .spawn((
                ControlParent(entity),
                Transform::from_translation(t.translation),
                Visibility::default(),
            ))
            .with_children(|parent| {
                parent
                    .spawn((
                        Transform::from_xyz(0.0, 0.0, 0.0).looking_to(Vec3::X, Vec3::Y),
                        Control(Vec3::X),
                        Visibility::default(),
                    ))
                    .with_children(|parent| {
                        draw_arrow(parent, red.clone(), red_hover.clone(), &mut meshes, scale);
                    })
                    .observe(drag_controller)
                    .observe(drag_controller3d)
                    .observe(drag_end_trigger_redraw)
                    .observe(drag_end3d_trigger_redraw);

                parent
                    .spawn((
                        Transform::from_xyz(0.0, 0.0, 0.0).looking_to(Vec3::Y, Vec3::Y),
                        Control(Vec3::Y),
                        Visibility::default(),
                    ))
                    .with_children(|parent| {
                        draw_arrow(
                            parent,
                            green.clone(),
                            green_hover.clone(),
                            &mut meshes,
                            scale,
                        );
                    })
                    .observe(drag_controller)
                    .observe(drag_controller3d)
                    .observe(drag_end_trigger_redraw)
                    .observe(drag_end3d_trigger_redraw);

                parent
                    .spawn((
                        Transform::from_xyz(0.0, 0.0, 0.0).looking_to(Vec3::Z, Vec3::Y),
                        Control(Vec3::Z),
                        Visibility::default(),
                    ))
                    .with_children(|parent| {
                        draw_arrow(parent, blue.clone(), blue_hover.clone(), &mut meshes, scale);
                    })
                    .observe(drag_controller)
                    .observe(drag_controller3d)
                    .observe(drag_end_trigger_redraw)
                    .observe(drag_end3d_trigger_redraw);
            });
    }
}

fn drag_end_trigger_redraw(
    _: Trigger<Pointer<DragEnd>>,
    mut redraw_writer: EventWriter<RedrawEvent>,
) {
    redraw_writer.write(RedrawEvent((400, 400)));
}

fn drag_end3d_trigger_redraw(
    _: Trigger<Pointer3d<picking_3d::DragEnd>>,
    mut redraw_writer: EventWriter<RedrawEvent>,
) {
    redraw_writer.write(RedrawEvent((400, 400)));
}

fn drag_controller(
    trigger: Trigger<Pointer<Drag>>,
    control_query: Query<(&Control, &ChildOf)>,
    mut all_other_transforms: Query<&mut Transform, (Without<Control>, Without<ControlParent>)>,
    camera: Query<(&Camera, &GlobalTransform)>,
    mut control_parents: Query<(&ControlParent, &mut Transform)>,
    mut redraw_writer: EventWriter<RedrawEvent>,
) {
    let (control, child_of) = control_query.get(trigger.target()).unwrap();

    let parent = child_of.parent();

    let (camera, camera_transform) = camera.single().unwrap();

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

    let axis = control.0;
    let direction = (diff.dot(axis)) / (diff.length() * axis.length());
    let translation = axis * direction * trigger.delta.length() * 0.01;

    let (control_parent, mut transform) = control_parents.get_mut(parent).unwrap();

    transform.translation = transform.translation + translation;

    let control_point = all_other_transforms.get_mut(control_parent.0);
    if control_point.is_ok() {
        let mut t = control_point.unwrap();
        t.translation = transform.translation;
    };

    redraw_writer.write(RedrawEvent((50, 50)));
}

fn drag_controller3d(
    trigger: Trigger<Pointer3d<picking_3d::Drag>>,
    control_query: Query<(&Control, &ChildOf)>,
    mut all_other_transforms: Query<&mut Transform, (Without<Control>, Without<ControlParent>)>,
    camera: Query<(&Camera, &GlobalTransform)>,
    mut control_parents: Query<(&ControlParent, &mut Transform)>,
    mut redraw_writer: EventWriter<RedrawEvent>,
) {
    println!("Dragging");
    let (control, child_of) = control_query.get(trigger.target()).unwrap();

    let parent = child_of.parent();

    let (camera, camera_transform) = camera.single().unwrap();

    let diff = { trigger.event.current_entity_position - trigger.event.start_entity_position };

    let axis = control.0;
    let direction = (diff.dot(axis)) / (diff.length() * axis.length());
    let translation = axis * direction * trigger.event.current_delta.length() * 0.01;

    let (control_parent, mut transform) = control_parents.get_mut(parent).unwrap();

    transform.translation = transform.translation + translation;

    let control_point = all_other_transforms.get_mut(control_parent.0);
    if control_point.is_ok() {
        let mut t = control_point.unwrap();
        t.translation = transform.translation;
    };

    redraw_writer.write(RedrawEvent((50, 50)));
}

pub struct TranslationController;

impl Plugin for TranslationController {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (show_transitional_controls));
        app.add_systems(PostUpdate, register_deletes);
    }
}
