//! The realm's Cover is walked through (spec #533): a GameObject in
//! [`crate::twilight::WalkThroughList`] has no body or hull collider. [`super::attach_entity_visuals`]
//! skips the hull for one listed when it builds; [`walk_through_cover`] follows the list's later
//! changes on objects already standing.

use avian3d::prelude::{Collider, RigidBody};
use benilla_protocol::EntityKind;
use bevy::prelude::*;

use crate::net::{Guid, NetEntity};

use super::super::{GameObjects, VisualAttached};

/// A newly listed GameObject loses its body and hull; one dropped from the list gets them back from
/// its display, which keeps building the collider. Runs only on a change of the list.
pub(in super::super) fn walk_through_cover(
    mut commands: Commands,
    list: Option<Res<crate::twilight::WalkThroughList>>,
    gameobjects: Option<Res<GameObjects>>,
    standing: Query<
        (
            Entity,
            &Guid,
            &NetEntity,
            Has<Collider>,
            Has<crate::transport::TransportAnchor>,
        ),
        With<VisualAttached>,
    >,
    mut collider_epoch: ResMut<benilla_world::collision::ColliderEpoch>,
) {
    let Some(list) = list.filter(|l| l.is_changed()) else {
        return;
    };
    let mut changed = false;
    for (entity, guid, net, has_collider, anchored) in &standing {
        if net.kind != EntityKind::GameObject {
            continue;
        }
        match (list.lists(guid.0), has_collider) {
            (true, true) => {
                commands.entity(entity).remove::<(RigidBody, Collider)>();
                changed = true;
            }
            (false, false) => {
                let hull = net
                    .display_id
                    .and_then(|d| gameobjects.as_deref()?.models.get(&d)?.collider.clone());
                let Some(col) = hull else {
                    continue;
                };
                // The body the attach lane picks: an anchored transport stays Kinematic.
                let body = if anchored {
                    RigidBody::Kinematic
                } else {
                    RigidBody::Static
                };
                commands.entity(entity).insert((body, col));
                changed = true;
            }
            _ => {}
        }
    }
    // Outside the streamer's attach queue, so the lane stamps the collider set itself.
    if changed {
        collider_epoch.bump();
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::super::super::{display::empty_shell, CubeAssets, ModelHandle, SkinComposites};
    use super::super::{attach_entity_visuals, merge};
    use super::*;
    use crate::twilight::WalkThroughList;
    use benilla_world::collision::ColliderEpoch;

    /// A display whose model has a collision hull, built by hand.
    const HULL: u32 = 6133;
    const GUID: u64 = 0xF110_0000_0000_0042;

    /// The attach chain with one loaded hull display and an empty walk-through list.
    fn app() -> App {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()))
            .init_asset::<Image>()
            .init_asset::<Mesh>()
            .init_asset::<benilla_assets::materials::WowModelMaterial>()
            .init_resource::<benilla_world::model_render::ModelMaterials>()
            .init_resource::<super::super::super::corpse::BonesModels>()
            .init_resource::<SkinComposites>()
            .init_resource::<merge::MergedFormsCache>()
            .init_resource::<benilla_world::doodad_anim::UvAnimMaterials>()
            .init_resource::<benilla_world::doodad_anim::TintAnimMaterials>()
            .init_resource::<benilla_world::mat_anim_table::MatAnimTable>()
            .init_resource::<benilla_world::rig_palette::RigPalettes>()
            .init_resource::<ColliderEpoch>()
            .init_resource::<WalkThroughList>()
            .insert_resource(CubeAssets {
                mesh: Handle::default(),
                player_mesh: Handle::default(),
                player_mat: Handle::default(),
                npc_mat: Handle::default(),
            })
            .insert_resource(GameObjects {
                catalog: Default::default(),
                models: HashMap::from([(
                    HULL,
                    super::super::super::DisplayModel {
                        handle: ModelHandle::M2(Handle::default()),
                        parts: Some(Vec::new()),
                        collider: Some(Collider::cuboid(1.0, 1.0, 1.0)),
                        ..empty_shell()
                    },
                )]),
            })
            .add_systems(Update, (attach_entity_visuals, walk_through_cover).chain());
        app
    }

    fn haystack(app: &mut App) -> Entity {
        app.world_mut()
            .spawn((
                NetEntity {
                    kind: EntityKind::GameObject,
                    display_id: Some(HULL),
                    scale: 1.0,
                },
                Guid(GUID),
                Transform::default(),
                Visibility::default(),
            ))
            .id()
    }

    fn list(app: &mut App, guids: Vec<u64>) {
        app.world_mut().resource_mut::<WalkThroughList>().0 = guids;
    }

    fn collides(app: &App, e: Entity) -> bool {
        let e = app.world().entity(e);
        e.contains::<Collider>() && e.contains::<RigidBody>()
    }

    fn epoch(app: &App) -> u64 {
        app.world().resource::<ColliderEpoch>().get()
    }

    #[test]
    fn listed_cover_attaches_without_a_collider() {
        let mut app = app();
        list(&mut app, vec![GUID]);
        let hay = haystack(&mut app);
        app.update();
        let e = app.world().entity(hay);
        assert!(e.contains::<VisualAttached>(), "the model still builds");
        assert!(!e.contains::<Collider>(), "a listed GameObject has no hull");
        assert!(!e.contains::<RigidBody>(), "nor a body");
    }

    #[test]
    fn an_unlisted_gameobject_with_a_hull_collides() {
        let mut app = app();
        let hay = haystack(&mut app);
        app.update();
        assert!(collides(&app, hay), "an unlisted hull collides as before");
        assert_eq!(
            app.world().get::<RigidBody>(hay),
            Some(&RigidBody::Static),
            "on the GameObject default body"
        );
    }

    #[test]
    fn listing_a_standing_object_drops_its_collider_and_an_empty_list_restores_it() {
        let mut app = app();
        let hay = haystack(&mut app);
        app.update();
        assert!(collides(&app, hay));

        let before = epoch(&app);
        list(&mut app, vec![GUID]);
        app.update();
        assert!(
            !app.world().entity(hay).contains::<Collider>(),
            "listing a standing object removes its hull"
        );
        assert!(!app.world().entity(hay).contains::<RigidBody>());
        assert!(epoch(&app) > before, "and stamps the collider set");

        let before = epoch(&app);
        list(&mut app, Vec::new());
        app.update();
        assert!(collides(&app, hay), "an empty list puts the hull back");
        assert!(epoch(&app) > before, "and stamps the collider set");

        // An unchanged list leaves the set alone.
        let before = epoch(&app);
        app.update();
        assert_eq!(epoch(&app), before);
    }
}
