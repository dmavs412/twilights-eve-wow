# Borrowed spells are rewritten in the client, not patched into an MPQ

A Calling may borrow a spell from another Class: Zealot casts Paladin Seals, Judgement and Hammer of Justice on a Warrior's rage bar. The client reads each spell's power type, cost and stance requirement from Spell.dbc, so it greys out a mana spell on a rage bar and never sends the cast. The Callings design assumed a stock client and asked for edited copies of those rows, under new spell IDs, in a patch MPQ.

The client is ours, and Twilight's Eve is its only mode. So when the client loads its spell data, it rewrites each borrowed spell's row in memory:

- the power type becomes the Calling's power type
- the cost becomes the skill's cost in the Local Realm's content
- any stance or shapeshift requirement is cleared
- the spell family becomes the character's Class, so the realm's talent spell modifiers reach the spell

The spell IDs stay the real ones. The realm's content marks which skills are borrowed, and the client reads that mark, so there is one source for every cost. The same hook appends " (WIP)" to the names of talents the realm does not model yet.

## Considered Options

- **A patch MPQ with copied rows under new IDs.** Rejected. It needs DBC-writing tools and a shipped data file. It also keeps two copies of every cost, one in the MPQ and one in the realm, that can drift apart.
- **Copies under new IDs, made in memory.** Rejected. It changes nothing for players, because nothing else in the game uses the Paladin rows. It also adds an ID mapping between the realm and the client's records.

## Consequences

- A future real server that plays to a stock client has to ship these rewrites as a patch MPQ after all. The realm's content already lists them.
- The rewritten rows apply to every character, so they assume Twilight's Eve is the only mode and that no Calling uses the original Paladin version of the same spell.
