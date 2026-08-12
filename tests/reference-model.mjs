import assert from 'node:assert/strict';

const LEVEL_BPS = [1500, 900, 600, 400, 250, 200, 150, 100, 100, 100];
const ACTIVE = 7 * 24 * 60 * 60;
const GRACE = 48 * 60 * 60;
const PIONEER_SLOTS = 100n;
const PIONEER_SCALE = 1_000_000_000_000_000_000n;

assert.equal(LEVEL_BPS.reduce((a, b) => a + b, 0), 4300);
assert.equal(5000 + 4300 + 200 + 500, 10000);

function split(amount) {
  const calc = bps => Math.floor(amount * bps / 10000);
  const levels = LEVEL_BPS.map(calc);
  const direct = calc(5000);
  const pioneer = calc(200);
  const service = calc(500);
  const allocated = direct + pioneer + service + levels.reduce((a, b) => a + b, 0);
  return { direct, levels, pioneer, service, remainder: amount - allocated };
}

function status(user, now) {
  if (user.activeUntil > 0 && now <= user.activeUntil) return 'ACTIVE';
  if (user.graceUntil > 0 && now <= user.graceUntil) return 'GRACE';
  return 'INACTIVE';
}

function purchaseUnits(user, units, now) {
  assert(units > 0);
  if (
    user.qualificationProgress > 0 &&
    user.qualificationWindowStartedAt > 0 &&
    now > user.qualificationWindowStartedAt + ACTIVE
  ) {
    user.qualificationProgress = 0;
    user.qualificationWindowStartedAt = 0;
  }

  if (user.qualificationProgress === 0) {
    user.qualificationWindowStartedAt = now;
  }
  user.qualificationProgress += units;

  if (user.qualificationProgress >= 10) {
    const wasGrace = status(user, now) === 'GRACE';
    user.qualificationProgress = 0;
    user.qualificationWindowStartedAt = 0;
    user.activeUntil = now + ACTIVE;
    user.graceUntil = user.activeUntil + GRACE;
    if (wasGrace) {
      user.claimable += user.pending;
      user.pending = 0;
    }
  }
}

function creditNetwork(user, amount, now) {
  const s = status(user, now);
  if (s === 'ACTIVE') user.claimable += amount;
  else if (s === 'GRACE') user.pending += amount;
  else user.expired += amount;
}

function settleExpired(user, now) {
  if (user.graceUntil > 0 && now > user.graceUntil && user.pending > 0) {
    const amount = user.pending;
    user.pending = 0;
    user.expired += amount;
    return amount;
  }
  return 0;
}

function newUser() {
  return {
    activeUntil: 0,
    graceUntil: 0,
    qualificationProgress: 0,
    qualificationWindowStartedAt: 0,
    claimable: 0,
    pending: 0,
    expired: 0,
  };
}

class PioneerIndex {
  constructor() {
    this.indexScaled = 0n;
    this.unassignedTreasuryScaled = 0n;
    this.treasuryAtomicTransferred = 0n;
  }

  accrue(poolAtomic, assignedSlots) {
    const pool = BigInt(poolAtomic);
    const assigned = BigInt(assignedSlots);
    assert(assigned >= 0n && assigned <= PIONEER_SLOTS);

    const perShareScaled = pool * PIONEER_SCALE / PIONEER_SLOTS;
    this.indexScaled += perShareScaled;

    const unassigned = PIONEER_SLOTS - assigned;
    this.unassignedTreasuryScaled += perShareScaled * unassigned;

    const wholeTreasury = this.unassignedTreasuryScaled / PIONEER_SCALE;
    this.unassignedTreasuryScaled %= PIONEER_SCALE;
    this.treasuryAtomicTransferred += wholeTreasury;
    return wholeTreasury;
  }

  due(checkpointScaled) {
    const diff = this.indexScaled - checkpointScaled;
    return diff / PIONEER_SCALE;
  }

  checkpointAfterClaim(checkpointScaled) {
    const due = this.due(checkpointScaled);
    return checkpointScaled + due * PIONEER_SCALE;
  }
}

// Exact 1 stablecoin split (6 decimals).
const one = split(1_000_000);
assert.deepEqual(one, {
  direct: 500_000,
  levels: [150_000, 90_000, 60_000, 40_000, 25_000, 20_000, 15_000, 10_000, 10_000, 10_000],
  pioneer: 20_000,
  service: 50_000,
  remainder: 0,
});

// Huge logical unit batch still fits a single u64 SPL transfer amount.
const hugeUnits = 100_000_000_000n;
const payment = hugeUnits * 1_000_000n;
assert(payment <= 18_446_744_073_709_551_615n);

// Partial activity progress cannot accumulate forever.
{
  const u = newUser();
  purchaseUnits(u, 5, 1_000);
  assert.equal(u.qualificationProgress, 5);
  purchaseUnits(u, 5, 1_000 + ACTIVE + 1);
  assert.equal(status(u, 1_000 + ACTIVE + 1), 'INACTIVE');
  assert.equal(u.qualificationProgress, 5); // old 5 expired; only the new 5 remain.
  purchaseUnits(u, 5, 1_000 + ACTIVE + 2);
  assert.equal(status(u, 1_000 + ACTIVE + 2), 'ACTIVE');
}

// ACTIVE -> GRACE -> reactivation vests pending.
{
  const u = newUser();
  purchaseUnits(u, 10, 10_000);
  const graceTime = u.activeUntil + 1;
  assert.equal(status(u, graceTime), 'GRACE');
  creditNetwork(u, 123, graceTime);
  assert.equal(u.pending, 123);
  purchaseUnits(u, 10, graceTime + 1);
  assert.equal(u.pending, 0);
  assert.equal(u.claimable, 123);
  assert.equal(status(u, graceTime + 1), 'ACTIVE');
}

// Missed grace physically becomes treasury-settleable expired value.
{
  const u = newUser();
  purchaseUnits(u, 10, 20_000);
  const graceTime = u.activeUntil + 1;
  creditNetwork(u, 456, graceTime);
  const expiredAt = u.graceUntil + 1;
  const treasuryTransfer = settleExpired(u, expiredAt);
  assert.equal(treasuryTransfer, 456);
  assert.equal(u.pending, 0);
  assert.equal(u.expired, 456);
}

// Inactive network rewards are immediately lost/expired.
{
  const u = newUser();
  creditNetwork(u, 789, 30_000);
  assert.equal(u.claimable, 0);
  assert.equal(u.pending, 0);
  assert.equal(u.expired, 789);
}

// Pioneer fractions persist instead of disappearing on tiny amounts.
{
  const p = new PioneerIndex();
  let checkpoint = 0n;
  for (let i = 0; i < 99; i++) {
    p.accrue(1n, 100);
    assert.equal(p.due(checkpoint), 0n);
  }
  p.accrue(1n, 100);
  assert.equal(p.due(checkpoint), 1n);
  checkpoint = p.checkpointAfterClaim(checkpoint);
  assert.equal(p.due(checkpoint), 0n);
}

// Unassigned Pioneer virtual shares accrue to treasury without losing fractions.
{
  const p = new PioneerIndex();
  for (let i = 0; i < 2; i++) p.accrue(1n, 50);
  assert.equal(p.treasuryAtomicTransferred, 1n);
  assert.equal(p.unassignedTreasuryScaled, 0n);
}

// Randomized split conservation.
for (let i = 1; i <= 10_000; i++) {
  const amount = Math.floor(Math.random() * 10_000_000_000) + 1;
  const s = split(amount);
  assert.equal(
    s.direct + s.pioneer + s.service + s.remainder + s.levels.reduce((a, b) => a + b, 0),
    amount,
  );
}

console.log('reference model: V0.9 invariants passed');
