# Ground patches are realm DynamicObjects, named after their own row

Miasma (`ground_area`) and the Consecration that Bulwark Charge leaves (`ground_effect`) lived only
inside the realm. They worked, but the client drew nothing on the ground, and every Consecration
tick was logged as Bulwark Charge (#210). The client already draws a DynamicObject: it arms the
spell's SpellVisual area model and kit from `DYNAMICOBJECT_SPELLID` (`benilla-app` `dest_fx.rs`).
Ticket #318 has the realm send one. This ADR fixes which spell the object names and who is sent
it, so the next ground patch follows the same path.

- **Every ground patch is a DynamicObject.** That covers each `ground_area` and each
  `ground_effect`. The realm creates the object when the patch lands, and destroys it when the
  patch ends or a new one from the same caster replaces it. Its lifetime is the realm's own, so
  talents such as Hallowed Ground's extra seconds count. Its fields are the ones vmangos's
  `DynamicObject::Create` fills: the caster, the spell id, the radius, the centre and the area
  spell type byte. A re-cast is a new guid. The client never re-creates an object in place.
- **The object names the patch's own CSV row.** A `ground_effect` may name a row with
  `ground_spell=key`. The build resolves that key to `ground_spell_id`, so the realm reads an id
  and never a key or a literal. The patch's object and its tick damage logs both carry that id.
  With no `ground_spell`, they carry the casting row's id. The named row must be client visible,
  because the client draws the object from that row's visual. It is usually `spellbook=no`.
  Bulwark Charge names `consecration`, a Vanguard row with the stock Consecration visual. Miasma
  names itself and takes the stock Death & Decay visual.
- **The caster's session streams its own patches. The shard view streams them to everyone
  else.** The caster's session sends the create and the destroy directly. Every other player on
  the same shard (the same Phase) gets the patch through the view code that streams units. A
  patch whose centre is in view is created, and one that leaves view or ends is removed. A player
  who walks up mid-patch is sent it, and a stealthed caster's patch still shows.

## Considered options

- **Name the object after the casting skill.** Rejected: Bulwark Charge's visual is the charge's,
  so the ground would show the wrong model, and the combat log would keep naming the charge.
- **Send the create to everyone in view at the moment it lands.** Rejected: a player who walks up
  a second later would never see the patch. Following the unit view keeps a late arrival right
  for free.
- **A realm entity for every patch.** Not needed yet: a patch lives in its caster's skills state,
  like the rest of the cast, and the view reads it from there each tick.
