"""Pulls of N mobs on top of model's Calling classes. The Calling's own code fights one mob (the
current target); this layer adds the other mobs' swings, routes AoE sources to several mobs and
moves the target on when one dies. A pull of two or more starts with every mob in melee, as
docs/balance/rules.md has it: all N reach melee within the first second."""
from __future__ import annotations

import model


class Multi:
    N = 1
    friend = None      # H-1's dummy friend in a Guard fight, else None
    AOE = {}           # damage source -> targets hit (0 = every living mob)
    ROLLS = set()      # AoE sources whose extra targets roll a 15% avoid
    PER_TARGET = {}    # attributes cleared when the target changes

    @property
    def mob_hp(self):
        return self.mobs[self.tgt]

    @mob_hp.setter
    def mob_hp(self, v):
        self.mobs[self.tgt] = v

    def alive(self):
        return [i for i, h in enumerate(self.mobs) if h > 0]

    def hit_mob(self, raw, src, *a, **kw):
        n = self.AOE.get(src)
        if n is None:
            return model.Sim.hit_mob(self, raw, src, *a, **kw)
        keep = self.tgt
        others = [i for i in self.alive() if i != keep]
        targets = [keep] + (others if n == 0 else others[:n - 1])
        d0 = 0
        for i in targets:
            if i != keep and src in self.ROLLS and self.rng.random() < 0.15:
                continue
            self.tgt = i
            d = model.Sim.hit_mob(self, raw, src, *a, **kw)
            if i == keep:
                d0 = d
        self.tgt = keep
        return d0

    def splash(self, d, src, n=1):
        others = [i for i in self.alive() if i != self.tgt]
        if d > 0 and others:
            keep = self.tgt
            for i in others if n == 0 else others[:n]:
                self.tgt = i
                model.Sim.hit_mob(self, d, src, physical=False)
            self.tgt = keep

    def extra_defended(self, i, o):
        """Mob `i`, not the target, swung at the character and it came out a block, a parry or a
        dodge (`o`)."""
        pass

    def retarget(self):
        for k, v in self.PER_TARGET.items():
            setattr(self, k, list(v) if isinstance(v, list) else v)

    def extra_step(self):
        if self.t < self.extra_swing_t:
            return
        self.extra_swing_t = self.t + self.m["swing"]
        for i in [i for i in self.alive() if i != self.tgt]:
            d, o = self.mob_swing(i=i)
            if o in ("block", "parry", "dodge"):
                self.extra_defended(i, o)
            if d > 0:
                d = self.take(d)
                if hasattr(self, "rage_taken"):
                    self.rage_taken(d)

    def after_step(self):
        pass

    def fight(self):
        """One fight from full health: won when every mob dies and the character lives."""
        s = self.s
        self.mobs = [self.m["hp"]] * self.N
        self.tgt = 0
        self.m_armor = self.m["armor"]
        self.taken = 0
        self.costs = 0
        t0 = self.t0 = self.t
        self.extra_swing_t = t0 + 0.5
        self.setup_fight()
        if self.N > 1 and getattr(self, "dist", 0) > 5:
            self.dist = 5.0
        while self.going():
            if self.t >= self.regen_next:
                self.regen_tick(True)
                self.regen_next += 2.0
            self.step()
            self.extra_step()
            self.after_step()
            if self.mobs[self.tgt] <= 0 and self.alive():
                self.tgt = self.alive()[0]
                self.retarget()
            self.t += model.DT
        won = self.hp > 0 and not self.alive()
        return dict(time=self.t - t0, won=won, died=self.hp <= 0, health_left=max(self.hp, 0) / s.max_hp)

    def going(self):
        """The fight goes on: a mob and the character live, and the cap has not come."""
        return self.alive() and self.hp > 0 and self.t - self.t0 < self.cap_s()

    def cap_s(self):
        return model.FIGHT_CAP_S


# H-1 (docs/balance/rules.md): how long the friend must live, and the share of its health it must keep.
GUARD_S = 60.0
GUARD_LEFT = 0.30


class Guard:
    """H-1's fight, ahead of a Multi Calling class: one mob swings at a dummy friend, never at the
    character, for GUARD_S seconds. The friend (`friend`: hp, armor) has no dodge, parry or block;
    a swing misses 5%, crits 5%, and passes the friend's armour. A mob that dies is replaced by a
    fresh one at once. A healer's heal reaches the friend through heal_friend. The fight is won
    when the friend and the character live to the end and the friend keeps GUARD_LEFT of its
    health; health_left is the friend's share."""

    def mob_swing(self, from_front=True, i=None):
        f = self.friend
        raw = self.rng.uniform(self.m["dmin"], self.m["dmax"])
        weaken = self.weaken_pct(i)
        if weaken:
            raw = raw * (100 - weaken) / 100
        d = model.armored(raw, f["armor"], self.m["level"])
        r = self.rng.random() * 100
        if r >= 5:
            self.hurt_friend(d * 2 if r < 10 else d)
        return 0, "friend"

    def hurt_friend(self, d):
        """`d` damage on the friend, past the shield a healer put on it (shield_friend)."""
        w = self.friend_shield
        if w and self.t < w[1]:
            soaked = min(w[0], d)
            w[0] -= soaked
            d -= soaked
        self.friend_hp -= d

    def shield_friend(self, amount, ms):
        """A shield of `amount` on the friend for `ms`, in place of the one before (Blood Ward)."""
        self.friend_shield = [int(amount), self.t + ms / 1000]

    def heal_friend(self, amount):
        """`amount` healed on the friend, up to its maximum. Returns what it healed."""
        before = self.friend_hp
        self.friend_hp = min(self.friend_hp + int(amount), self.friend["hp"])
        self.friend_healed += self.friend_hp - before
        return self.friend_hp - before

    def after_step(self):
        super().after_step()
        if self.mobs[self.tgt] <= 0:
            self.mobs[self.tgt] = self.m["hp"]
            self.retarget()

    def going(self):
        return self.friend_hp > 0 and self.hp > 0 and self.t - self.t0 < GUARD_S

    def fight(self):
        self.friend_hp = self.friend["hp"]
        self.friend_shield = None
        self.friend_healed = 0
        r = super().fight()
        left = max(self.friend_hp, 0) / self.friend["hp"]
        lived = self.friend_hp > 0 and self.hp > 0
        r.update(won=lived and left >= GUARD_LEFT, died=not lived, health_left=left)
        return r


class BasicWarriorN(Multi, model.BasicWarrior):
    pass


class VanguardN(Multi, model.Vanguard):
    AOE = {"cleave": 3, "conviction": 3, "consecration": 0}
    ROLLS = {"cleave"}

    def extra_defended(self, i, o):
        # The procs' damage goes to the mob that swung.
        keep, self.tgt = self.tgt, i
        self.fire(o)
        self.tgt = keep


class CutthroatN(Multi, model.Cutthroat):
    PER_TARGET = dict(rupture=None, garrote=None)

    def extra_defended(self, i, o):
        if o == "dodge":
            self.dodged()


class InitiateN(Multi, model.Initiate):
    AOE = {"ward_burst": 0}
    PER_TARGET = dict(ember=None, cold_until=-1, slow_until=-1)

    def setup_fight(self):
        super().setup_fight()
        self.copies = {}   # mob -> [Ember, Cold's end] a consume copied there (Catalytic Chain)

    def spread(self, ember=False, cold=False):
        # A fresh copy, one stack and the aura's full duration, on the next living mob.
        others = [i for i in self.alive() if i != self.tgt]
        if others:
            copy = self.copies.setdefault(others[0], [None, -1])
            if ember:
                copy[0] = self.new_ember(1)
            if cold:
                copy[1] = self.t + self.cold_aura["duration_ms"] / 1000

    def retarget(self):
        super().retarget()
        self.ember, self.cold_until = self.copies.pop(self.tgt, [None, -1])
        self.slow_until = self.cold_until

    def weaken_pct(self, i=None):
        if i is None or i == self.tgt:
            return super().weaken_pct()
        return self.burn_weaken(self.copies.get(i, [None])[0])

    def after_step(self):
        for i, (ember, _) in self.copies.items():
            if self.mobs[i] > 0 and self.burning(ember) and self.t >= ember[1]:
                keep, self.tgt = self.tgt, i
                self.tick_ember(ember)
                self.tgt = keep


class RangerN(Multi, model.Ranger):
    AOE = {"explosive_shot": 0}
    PER_TARGET = dict(barbed=None, slow_until=-1, sunder=None)


class AcolyteN(Multi, model.Acolyte):
    AOE = {"miasma": 0, "dark_surplus": 0}

    @property
    def blight(self):
        return self.blights.get(self.tgt)

    @blight.setter
    def blight(self, b):
        self.blights[self.tgt] = b

    def setup_fight(self):
        self.blights = {}  # mob -> its Blight: the target's, and the copies a tick spread (Epidemic)
        super().setup_fight()

    def spread_blight(self, b):
        # A copy with its stacks and time left on the next living mob that wears none, ticking a
        # tick from now.
        bare = [i for i in self.alive() if i != self.tgt and not self.blighted(self.blights.get(i))]
        if bare:
            self.blights[bare[0]] = [b[0], self.t + self.blight_dot["tick_ms"] / 1000, b[2], b[3]]

    def grow_others(self, n=1):
        for i in self.alive():
            if i != self.tgt:
                self.grow(self.blights.get(i), n)

    def weaken_pct(self, i=None):
        return self.blight_weaken(self.blights.get(self.tgt if i is None else i))

    def after_step(self):
        # The Blights on the other mobs tick there.
        keep = self.tgt
        for i in self.alive():
            b = self.blights.get(i)
            if i != keep and self.blighted(b) and self.t >= b[1]:
                self.tgt = i
                self.tick_blight(b)
        self.tgt = keep


class IroncladN(Multi, model.Ironclad):
    def extra_defended(self, i, o):
        # A block's procs hit the mob that swung.
        keep, self.tgt = self.tgt, i
        self.fire(o)
        self.tgt = keep


class ZealotN(Multi, model.Zealot):
    AOE = {"consecration": 0, "hallowed_ground": 0, "holy_wrath": 0}

    def extra_defended(self, i, o):
        keep, self.tgt = self.tgt, i
        self.fire(o)
        self.tgt = keep


class BlightcallerN(Multi, model.Blightcaller):
    AOE = {"miasma": 0}

    @property
    def blight(self):
        return self.blights.get(self.tgt)

    @blight.setter
    def blight(self, b):
        self.blights[self.tgt] = b

    def setup_fight(self):
        self.blights = {}  # mob -> its Blight
        super().setup_fight()

    def bare_other(self):
        bare = [i for i in self.alive() if i != self.tgt and not self.blighted(self.blights.get(i))]
        return bare[0] if bare else None

    def spread_blight(self, b, n):
        # Copies with its stacks and time left, ticking a tick from now.
        bare = [i for i in self.alive() if i != self.tgt and not self.blighted(self.blights.get(i))][:n]
        for i in bare:
            self.blights[i] = [b[0], self.t + self.blight_dot["tick_ms"] / 1000, b[2], b[3]]
        return len(bare)

    def grow_others(self, n=1):
        for i in self.alive():
            if i != self.tgt:
                self.grow(self.blights.get(i), n)

    def after_step(self):
        keep = self.tgt
        # A mob that died wearing the Blight passes it on with every stack (Contagion), then loses it.
        for i, b in list(self.blights.items()):
            if self.mobs[i] <= 0:
                del self.blights[i]
                if self.blighted(b) and any(p["on"] == "wearer_death" for p in self.procs):
                    bare = [j for j in self.alive() if not self.blighted(self.blights.get(j))]
                    if bare:
                        self.blights[bare[0]] = [b[0], b[1], b[2], b[3]]
        # The Blights on the other mobs tick there.
        for i in self.alive():
            b = self.blights.get(i)
            if i != keep and self.blighted(b) and self.t >= b[1]:
                self.tgt = i
                self.tick_blight(b)
        self.tgt = keep
        super().after_step()


class BloodmenderN(Multi, model.Bloodmender):
    pass


CLASSES = dict(basic_warrior=BasicWarriorN, vanguard=VanguardN, cutthroat=CutthroatN, initiate=InitiateN,
               ranger=RangerN, acolyte=AcolyteN, ironclad=IroncladN, zealot=ZealotN, blightcaller=BlightcallerN,
               bloodmender=BloodmenderN)


def fighter(calling, n=1):
    """`calling`'s fight class against a pull of `n`."""
    return type(f"{calling}_x{n}", (CLASSES[calling],), {"N": n})


def guard(calling, friend):
    """`calling`'s H-1 fight class: one mob on the dummy `friend` (hp, armor)."""
    return type(f"{calling}_guard", (Guard, CLASSES[calling]), {"N": 1, "friend": friend})
