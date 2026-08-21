import assert from 'node:assert/strict';

// Production economics: buyer/SELF receives 50%; the 43% network pool spans
// exactly nine uplines beginning with the immutable sponsor.
const LEVEL_BPS = [1500, 900, 600, 400, 250, 200, 150, 100, 200];
const ACTIVE = 7 * 24 * 60 * 60;
const GRACE = 48 * 60 * 60;
const ACTIVE_WEEK_REQUIREMENTS = [10, 20, 30, 40, 50];
const ACTIVE_WEEK_TIER_SPAN = 2;
const DEPTH_THRESHOLDS = [10, 25, 50, 100, 200, 350, 500];
const PIONEER_SLOTS = 100n;
const PIONEER_SCALE = 1_000_000_000_000_000_000n;

assert.equal(LEVEL_BPS.length, 9);
assert.equal(LEVEL_BPS.reduce((a, b) => a + b, 0), 4300);
assert.equal(5000 + 4300 + 200 + 500, 10000);

function split(amount) {
  const calc = bps => Math.floor(amount * bps / 10000);
  const levels = LEVEL_BPS.map(calc);
  const selfReward = calc(5000);
  const pioneer = calc(200);
  const service = calc(500);
  const allocated = selfReward + pioneer + service + levels.reduce((a, b) => a + b, 0);
  return { selfReward, levels, pioneer, service, remainder: amount - allocated };
}

function status(user, now) {
  if (user.activeUntil > 0 && now <= user.activeUntil) return 'ACTIVE';
  if (user.graceUntil > 0 && now <= user.graceUntil) return 'GRACE';
  return 'INACTIVE';
}

function activeRequirementForWeek(weekNumber) {
  const normalized = Math.max(1, weekNumber);
  const tier = Math.floor((normalized - 1) / ACTIVE_WEEK_TIER_SPAN);
  return ACTIVE_WEEK_REQUIREMENTS[Math.min(tier, ACTIVE_WEEK_REQUIREMENTS.length - 1)];
}

function nextActiveRequirement(activeWeeksStarted) {
  return activeRequirementForWeek(activeWeeksStarted + 1);
}

function networkDepthForUnits(units) {
  let depth = 0;
  for (let i = 0; i < DEPTH_THRESHOLDS.length; i++) {
    if (units < DEPTH_THRESHOLDS[i]) break;
    depth = i + 3;
  }
  return depth;
}

function purchaseUnits(user, units, now) {
  assert(units > 0);
  const preStatus = status(user, now);

  // Purchases during an already ACTIVE cycle increase current-week depth only.
  // They never pre-qualify the following ACTIVE week.
  if (preStatus === 'ACTIVE') {
    user.currentWeekUnits += units;
    return;
  }

  // The existence sentinel is progress > 0, not timestamp > 0. Timestamp zero is
  // valid in deterministic test environments and must not resurrect stale progress.
  if (
    user.qualificationProgress > 0 &&
    now > user.qualificationWindowStartedAt + ACTIVE
  ) {
    user.qualificationProgress = 0;
    user.qualificationWindowStartedAt = 0;
  }

  if (user.qualificationProgress === 0) {
    user.qualificationWindowStartedAt = now;
  }
  user.qualificationProgress += units;

  const required = nextActiveRequirement(user.activeWeeksStarted);
  if (user.qualificationProgress >= required) {
    const activatingUnits = user.qualificationProgress;
    user.qualificationProgress = 0;
    user.qualificationWindowStartedAt = 0;
    user.activeWeeksStarted += 1;
    user.currentWeekUnits = activatingUnits;
    user.activeUntil = now + ACTIVE;
    user.graceUntil = user.activeUntil + GRACE;
    if (preStatus === 'GRACE') {
      user.claimable += user.pending;
      user.pending = 0;
    }
  }
}

// Mirrors the on-chain no-compression rule. ACTIVE/GRACE users only monetize a
// scheduled network level when current-week personal units unlock that depth.
function creditNetwork(user, amount, levelNumber, now) {
  const s = status(user, now);
  if ((s === 'ACTIVE' || s === 'GRACE') && networkDepthForUnits(user.currentWeekUnits) < levelNumber) {
    user.unallocated += amount;
    return;
  }
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
    activeWeeksStarted: 0,
    currentWeekUnits: 0,
    qualificationProgress: 0,
    qualificationWindowStartedAt: 0,
    claimable: 0,
    pending: 0,
    expired: 0,
    unallocated: 0,
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

  due(checkpointScaled, positions = 1n) {
    const entitlement = this.indexScaled * BigInt(positions);
    const diff = entitlement - checkpointScaled;
    return diff / PIONEER_SCALE;
  }

  checkpointAfterClaim(checkpointScaled, positions = 1n) {
    const due = this.due(checkpointScaled, positions);
    return checkpointScaled + due * PIONEER_SCALE;
  }
}

// Exact 1 stablecoin split (6 decimals).
const one = split(1_000_000);
assert.deepEqual(one, {
  selfReward: 500_000,
  levels: [150_000, 90_000, 60_000, 40_000, 25_000, 20_000, 15_000, 10_000, 20_000],
  pioneer: 20_000,
  service: 50_000,
  remainder: 0,
});

// Huge logical unit batch still fits a single u64 SPL transfer amount.
const hugeUnits = 100_000_000_000n;
const payment = hugeUnits * 1_000_000n;
assert(payment <= 18_446_744_073_709_551_615n);

// Frozen progressive ACTIVE requirements and the permanent 50-unit cap.
{
  const expected = [10, 10, 20, 20, 30, 30, 40, 40, 50, 50, 50, 50];
  expected.forEach((required, index) => {
    assert.equal(activeRequirementForWeek(index + 1), required);
  });
  assert.equal(activeRequirementForWeek(10_000), 50);
}

// Exact weekly personal-unit depth boundaries.
{
  const expected = [
    [0, 0], [9, 0], [10, 3], [24, 3], [25, 4], [49, 4],
    [50, 5], [99, 5], [100, 6], [199, 6], [200, 7], [349, 7],
    [350, 8], [499, 8], [500, 9], [10_000, 9],
  ];
  for (const [units, depth] of expected) assert.equal(networkDepthForUnits(units), depth);
}

// Partial activity progress cannot accumulate forever, including a qualification
// window that begins exactly at timestamp zero.
{
  const u = newUser();
  purchaseUnits(u, 5, 0);
  assert.equal(u.qualificationProgress, 5);
  purchaseUnits(u, 5, ACTIVE + 1);
  assert.equal(status(u, ACTIVE + 1), 'INACTIVE');
  assert.equal(u.qualificationProgress, 5);
  purchaseUnits(u, 5, ACTIVE + 2);
  assert.equal(status(u, ACTIVE + 2), 'ACTIVE');
  assert.equal(u.activeWeeksStarted, 1);
}

// ACTIVE -> GRACE -> reactivation vests pending; week 2 still requires 10.
{
  const u = newUser();
  purchaseUnits(u, 10, 10_000);
  assert.equal(u.activeWeeksStarted, 1);
  const graceTime = u.activeUntil + 1;
  assert.equal(status(u, graceTime), 'GRACE');
  creditNetwork(u, 123, 1, graceTime);
  assert.equal(u.pending, 123);
  purchaseUnits(u, 10, graceTime + 1);
  assert.equal(u.pending, 0);
  assert.equal(u.claimable, 123);
  assert.equal(status(u, graceTime + 1), 'ACTIVE');
  assert.equal(u.activeWeeksStarted, 2);
  assert.equal(nextActiveRequirement(u.activeWeeksStarted), 20);
}

// Long calendar inactivity does not advance the ACTIVE-week counter. Week 3 still
// requires 20, and 19+1 inside one live qualification window activates exactly once.
{
  const u = newUser();
  purchaseUnits(u, 10, 1000);
  purchaseUnits(u, 10, u.activeUntil + 1);
  assert.equal(u.activeWeeksStarted, 2);
  const muchLater = u.graceUntil + 30 * 24 * 60 * 60;
  purchaseUnits(u, 19, muchLater);
  assert.equal(u.activeWeeksStarted, 2);
  assert.equal(u.qualificationProgress, 19);
  purchaseUnits(u, 1, muchLater + 1);
  assert.equal(u.activeWeeksStarted, 3);
  assert.equal(u.currentWeekUnits, 20);
}

// While ACTIVE, extra purchase volume increases depth prospectively but does not
// prequalify another ACTIVE week.
{
  const u = newUser();
  purchaseUnits(u, 10, 50_000);
  assert.equal(u.activeWeeksStarted, 1);
  assert.equal(networkDepthForUnits(u.currentWeekUnits), 3);
  creditNetwork(u, 40, 4, 50_001);
  assert.equal(u.claimable, 0);
  assert.equal(u.unallocated, 40);
  purchaseUnits(u, 490, 50_002);
  assert.equal(u.activeWeeksStarted, 1);
  assert.equal(u.currentWeekUnits, 500);
  assert.equal(networkDepthForUnits(u.currentWeekUnits), 9);
  creditNetwork(u, 20, 9, 50_003);
  assert.equal(u.claimable, 20);
}

// Missed grace physically becomes treasury-settleable expired network value.
{
  const u = newUser();
  purchaseUnits(u, 10, 20_000);
  const graceTime = u.activeUntil + 1;
  creditNetwork(u, 456, 1, graceTime);
  const expiredAt = u.graceUntil + 1;
  const treasuryTransfer = settleExpired(u, expiredAt);
  assert.equal(treasuryTransfer, 456);
  assert.equal(u.pending, 0);
  assert.equal(u.expired, 456);
}

// Inactive network rewards are immediately lost/expired.
{
  const u = newUser();
  creditNetwork(u, 789, 1, 30_000);
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

// Weighted Pioneer positions share one wallet checkpoint without losing precision.
{
  const p = new PioneerIndex();
  p.accrue(100n, 100);
  const twoPositionsDebtAtEntry = p.indexScaled * 2n;
  p.accrue(100n, 100);
  assert.equal(p.due(twoPositionsDebtAtEntry, 2n), 2n);
}

// Unassigned Pioneer virtual shares accrue to treasury without losing fractions.
{
  const p = new PioneerIndex();
  for (let i = 0; i < 2; i++) p.accrue(1n, 50);
  assert.equal(p.treasuryAtomicTransferred, 1n);
  assert.equal(p.unassignedTreasuryScaled, 0n);
}

for (let i = 1; i <= 10_000; i++) {
  const amount = Math.floor(Math.random() * 10_000_000_000) + 1;
  const s = split(amount);
  assert.equal(
    s.selfReward + s.pioneer + s.service + s.remainder + s.levels.reduce((a, b) => a + b, 0),
    amount,
  );
}

console.log('reference model: V0.14 progressive ACTIVE/depth + Pioneer invariants passed');
