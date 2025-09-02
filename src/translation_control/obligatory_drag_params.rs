use bevy::{
    color::palettes::tailwind::{YELLOW_400, YELLOW_600},
    ecs::system::{
        SystemParam,
        lifetimeless::{Read, Write},
    },
    prelude::*,
};

use crate::{
    bezier_curve::{
        bezier_curve_renderer::RedrawEvent,
        helper_curves::{CurveCollection, RedrawCurvesEvent},
        render_info::RenderInformation,
    },
    nurbs::{
        bezier::{de_casteljau, derive_after_de_casteljau},
        point::Point,
    },
    projection::ProjectedSnappingDetector,
};

use super::translation_controller::{
    CantSnapToCurve, Control, ControlParent, MovedEntityEvent, SnappedArrow, SnappedPoint,
    SnappingBehaviour, TranslationControllerState, drag_controller, drag_controller3d, draw_arrow,
};

#[allow(clippy::complexity)]
#[derive(SystemParam)]
pub struct ObligatoryDragParams<'w, 's> {
    commands: Commands<'w, 's>,
    transform_set: ParamSet<
        'w,
        's,
        (
            Query<'w, 's, Write<Transform>, (Without<Control>, Without<ControlParent>)>,
            CurveCollection<'w, 's>,
            ProjectedSnappingDetector<'w, 's>,
        ),
    >,
    redraw_writer: EventWriter<'w, RedrawEvent>,
    redraw_curves_writer: EventWriter<'w, RedrawCurvesEvent>,
    snapped: Query<'w, 's, (Entity, Write<SnappedPoint>)>,
    info: Res<'w, RenderInformation>,
    state: Res<'w, TranslationControllerState>,
    materials: ResMut<'w, Assets<StandardMaterial>>,
    meshes: ResMut<'w, Assets<Mesh>>,
    cant_snap_to_curve: Query<'w, 's, Read<CantSnapToCurve>>,
    moved_entity_writer: EventWriter<'w, MovedEntityEvent>,
}

impl<'w, 's> ObligatoryDragParams<'w, 's> {
    fn snap_to_projection(
        &mut self,
        control_parent: (Entity, &ControlParent),
        translation: Vec3,
        closest_move_direction: Vec3,
    ) -> Vec3 {
        self.commands
            .get_entity(control_parent.1.0)
            .unwrap()
            .insert(SnappedPoint::ToProjection);

        // Move to the snapped orthogonal projection
        let mut p0 = self.transform_set.p0();
        let mut t = p0.get_mut(control_parent.1.0).unwrap();

        t.translation = t.translation + translation + closest_move_direction;
        t.translation
    }

    fn snap_to_curve(
        &mut self,
        control_parent: (Entity, &ControlParent),
        t: Transform,
        translation: Vec3,
        cant_snap_to_curve: Option<CantSnapToCurve>,
    ) -> Vec3 {
        let shortest = self.transform_set.p1().collect_shortest(
            Point::from(t.translation + translation),
            &cant_snap_to_curve.unwrap_or_default(),
        );

        if let Some((curve, u, p, dist, points)) = shortest
            && dist < 0.05 * self.info.scale as f64
        {
            let mut p0 = self.transform_set.p0();
            let mut t = p0.get_mut(control_parent.1.0).unwrap();
            t.translation = p.into();

            let mat = self.materials.add(StandardMaterial::from_color(YELLOW_600));
            let mat_hover = self.materials.add(StandardMaterial::from_color(YELLOW_400));

            let deriv = derive_after_de_casteljau(&de_casteljau(&points, u), 1);

            self.commands
                .get_entity(control_parent.1.0)
                .unwrap()
                .insert(SnappedPoint::ToCurve { u, curve });

            // Create a new arrow (directional), that follows the curvature of the
            // curve that the point was snapped to. The direction of the arrow is
            // updated each frame, to adhere to movement along the curve
            self.commands
                .get_entity(control_parent.0)
                .unwrap()
                .with_children(|cmd| {
                    cmd.spawn((
                        Control(Vec3::from(deriv)),
                        SnappedArrow,
                        Transform::default().looking_to(Vec3::from(deriv), Vec3::Y),
                        Visibility::Inherited,
                    ))
                    .with_children(|cmd| {
                        draw_arrow(
                            cmd,
                            mat,
                            mat_hover,
                            &mut self.meshes,
                            self.info.scale,
                            false,
                        );
                    })
                    .observe(drag_controller)
                    .observe(drag_controller3d);
                });
            t.translation
        } else {
            // all curves are too far away to snap to
            let mut p0 = self.transform_set.p0();
            let mut t = p0.get_mut(control_parent.1.0).unwrap();
            t.translation += translation;
            t.translation
        }
    }

    fn try_create_snapping(
        &mut self,
        control_parent: (Entity, &ControlParent),
        t: Transform,
        translation: Vec3,
        cant_snap_to_curve: Option<CantSnapToCurve>,
    ) -> Vec3 {
        if let Some(closest_move_direction) = self
            .transform_set
            .p2()
            .detect_closest_projected(control_parent.1.0, translation)
            && closest_move_direction.length() < 0.05 * self.info.scale
        {
            self.snap_to_projection(control_parent, translation, closest_move_direction)
        } else {
            self.snap_to_curve(control_parent, t, translation, cant_snap_to_curve)
        }
    }

    fn check_and_process_already_snapped(
        &mut self,
        control_parent: (Entity, &ControlParent),
        t: Transform,
        translation: Vec3,
        cant_snap_to_curve: Option<CantSnapToCurve>,
        snap: SnappedPoint,
    ) -> Vec3 {
        match snap {
            SnappedPoint::ToCurve { u, curve } => {
                let point = Point::from(t.translation + translation);

                let shortest = self
                    .transform_set
                    .p1()
                    .collect_shortest(point, &cant_snap_to_curve.unwrap_or_default());
                if let Some((curve, u, p, dist, _)) = shortest
                    && dist < 0.05 * self.info.scale as f64
                {
                    let mut snap = self.snapped.get_mut(control_parent.1.0).unwrap().1;
                    *snap = SnappedPoint::ToCurve { u, curve };

                    let mut p0 = self.transform_set.p0();
                    let mut t = p0.get_mut(control_parent.1.0).unwrap();
                    t.translation = p.into();
                    t.translation
                } else {
                    let entity = self.snapped.get(control_parent.1.0).unwrap().0;
                    self.commands
                        .get_entity(entity)
                        .unwrap()
                        .remove::<SnappedPoint>();

                    // Once removed (Snapped point, consider adding it back when snapping to a
                    // projected position)
                    if let Some(closest_move_direction) = self
                        .transform_set
                        .p2()
                        .detect_closest_projected(control_parent.1.0, translation)
                        && closest_move_direction.length() < 0.05 * self.info.scale
                    {
                        self.snap_to_projection(control_parent, translation, closest_move_direction)
                    } else {
                        let mut p0 = self.transform_set.p0();
                        let mut t = p0.get_mut(control_parent.1.0).unwrap();
                        t.translation = point.into();
                        t.translation
                    }
                }
            }
            SnappedPoint::ToProjection => {
                // Once removed (Snapped point, consider adding it back when snapping to a
                // projected position)
                if let Some(closest_move_direction) = self
                    .transform_set
                    .p2()
                    .detect_closest_projected(control_parent.1.0, translation)
                    && closest_move_direction.length() < 0.05 * self.info.scale
                {
                    let mut p0 = self.transform_set.p0();
                    let mut t = p0.get_mut(control_parent.1.0).unwrap();
                    t.translation = t.translation + translation + closest_move_direction;
                    t.translation
                } else {
                    // Remove snapped component
                    let entity = self.snapped.get(control_parent.1.0).unwrap().0;
                    self.commands
                        .get_entity(entity)
                        .unwrap()
                        .remove::<SnappedPoint>();

                    // Consider adding back the curve when not
                    self.snap_to_curve(control_parent, t, translation, cant_snap_to_curve)
                }
            }
        }
    }

    pub fn update_position_drag_universal(
        &mut self,
        control_parent: (Entity, &ControlParent),
        translation: Vec3,
    ) {
        // Only adjust the control parent
        let is_already_snapped = self.snapped.get(control_parent.1.0).is_ok();
        let cant_snap_to_curve = self
            .cant_snap_to_curve
            .get(control_parent.1.0)
            .ok()
            .cloned();

        let changed_entity = control_parent.1.0;
        let control_point = self.transform_set.p0().get(control_parent.1.0).copied();
        if let Ok(t) = control_point {
            let started_translation = t.translation;

            let ending_translation = if self.state.curve_snapping == SnappingBehaviour::Snap {
                if !is_already_snapped {
                    self.try_create_snapping(control_parent, t, translation, cant_snap_to_curve)
                } else {
                    // The entity is already snapped to a curve. Either continue snapping by moving
                    // the entity back to the curve, or if the distance is to large, remove the
                    // snapping
                    let snap = self
                        .snapped
                        .get(control_parent.1.0)
                        .ok()
                        .map(|v| *v.1)
                        .expect(
                            "This is already checked in this branch by is_already_snapped == true",
                        );

                    self.check_and_process_already_snapped(
                        control_parent,
                        t,
                        translation,
                        cant_snap_to_curve,
                        snap,
                    )
                }
            } else {
                let mut p0 = self.transform_set.p0();
                let mut t = p0.get_mut(control_parent.1.0).unwrap();
                t.translation += translation;
                t.translation
            };

            // Trigger the moved event, so that linked entities may update the position of the
            // linked entity, correspondingly (using lokal transforms)
            if let Ok(mut entity) = self.commands.get_entity(changed_entity) {
                let delta = ending_translation - started_translation;
                entity.trigger(MovedEntityEvent {
                    entity: changed_entity,
                    delta,
                });
                self.moved_entity_writer.write(MovedEntityEvent {
                    entity: changed_entity,
                    delta,
                });
            }
        }

        self.redraw_writer.write(RedrawEvent::Fast);
        self.redraw_curves_writer.write(RedrawCurvesEvent);
    }
}
