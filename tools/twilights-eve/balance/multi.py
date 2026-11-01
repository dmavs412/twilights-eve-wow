"""Pulls of N mobs on top of model's Calling classes. The Calling's own code fights one mob (the
current target); this layer adds the other mobs' swings, routes AoE sources to several mobs and
moves the target on when one dies. A pull of two or more starts with every mob in melee, as
docs/balance/rules.md has it: all N reach melee within the first second."""
from __future__ import annotations

import model


class Multi:
    N = 1
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
        while self.alive() and self.hp > 0 and self.t - t0 < self.cap_s():
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

    def cap_s(self):
        return model.FIGHT_CAP_S


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


CLASSES = dict(basic_warrior=BasicWarriorN, vanguard=VanguardN, cutthroat=CutthroatN, initiate=InitiateN,
               ranger=RangerN, acolyte=AcolyteN)


def fighter(calling, n=1):
    """`calling`'s fight class against a pull of `n`."""
    return type(f"{calling}_x{n}", (CLASSES[calling],), {"N": n})
