//! The talent frame follows the current Calling: whenever the character in the world has a
//! different Calling than the catalog was built for (login, and Advancement later), the client's
//! talent catalog is replaced with that Calling's Talent Tree, its own TalentTab and Talent rows
//! in the compiled-in client patch (ADR 0011). `feed_talents` rebuilds the frame when the
//! resource changes.

use benilla_formats::{TalentCatalog, CLIENT_DBC_PATCH};
use bevy::prelude::*;

use super::CallingKeys;
use crate::net::SelfGuid;
use crate::ui_talent::Talents;

/// Puts the current Calling's tree in the talent catalog when it changes.
pub(super) fn follow_calling(
    keys: Res<CallingKeys>,
    me: Res<SelfGuid>,
    mut commands: Commands,
    mut built_for: Local<Option<&'static str>>,
) {
    // The realm names the Calling, in-process or on a friends server alike.
    let Some(calling) = me.0.and_then(|guid| keys.of(guid)) else {
        return;
    };
    if *built_for == Some(calling.key.as_str()) {
        return;
    }
    *built_for = Some(calling.key.as_str());
    let catalog = match TalentCatalog::from_dbc_patch(CLIENT_DBC_PATCH, &calling.key) {
        Ok(Some(catalog)) => catalog,
        Ok(None) => {
            warn!(
                "twilight: the client patch has no talent tabs for {}",
                calling.key
            );
            TalentCatalog::from_parts(Vec::new(), Vec::new())
        }
        Err(err) => {
            error!("client_dbc_patch.json does not parse: {err}");
            return;
        }
    };
    info!(
        "twilight: the talent frame shows {}'s tree ({} talents)",
        calling.key,
        catalog.len()
    );
    commands.insert_resource(Talents { catalog });
}
