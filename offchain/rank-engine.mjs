// Deterministic V1 rank/badge engine.
//
// This module is intentionally off-chain. It consumes rolling 30-day gross network
// volume grouped by immutable direct-referral leg. It never changes or participates
// in the core 50/43/2/5 payout path.

export const QUALIFIED_LEG_MIN_QNV = 100n;

export const RANKS = Object.freeze([
  Object.freeze({ id: "bronze", name: "Bronze", qnv: 1_000n, qualifiedLegs: 2, maxLegBps: null }),
  Object.freeze({ id: "silver", name: "Silver", qnv: 5_000n, qualifiedLegs: 2, maxLegBps: null }),
  Object.freeze({ id: "gold", name: "Gold", qnv: 15_000n, qualifiedLegs: 3, maxLegBps: 6_000 }),
  Object.freeze({ id: "platinum", name: "Platinum", qnv: 50_000n, qualifiedLegs: 4, maxLegBps: 5_000 }),
  Object.freeze({ id: "emerald", name: "Emerald", qnv: 150_000n, qualifiedLegs: 5, maxLegBps: 5_000 }),
  Object.freeze({ id: "diamond", name: "Diamond", qnv: 500_000n, qualifiedLegs: 6, maxLegBps: 5_000 }),
  Object.freeze({ id: "double_diamond", name: "Double Diamond", qnv: 1_500_000n, qualifiedLegs: 6, maxLegBps: 4_500 }),
  Object.freeze({ id: "crown", name: "Crown", qnv: 5_000_000n, qualifiedLegs: 8, maxLegBps: 4_000 }),
  Object.freeze({ id: "crown_ambassador", name: "Crown Ambassador", qnv: 15_000_000n, qualifiedLegs: 10, maxLegBps: 4_000 }),
]);

const BPS_DENOMINATOR = 10_000n;
const RANK_INDEX = new Map(RANKS.map((rank, index) => [rank.id, index]));

function asNonNegativeBigInt(value, label) {
  const v = typeof value === "bigint" ? value : BigInt(value);
  if (v < 0n) throw new RangeError(`${label} must be non-negative`);
  return v;
}

function sum(values) {
  return values.reduce((total, value) => total + value, 0n);
}

export function rankEvidenceFor(rank, legVolumes) {
  const volumes = legVolumes.map((value, index) =>
    asNonNegativeBigInt(value, `legVolumes[${index}]`),
  );
  const totalQnv = sum(volumes);
  const qualifiedLegCount = volumes.filter((v) => v >= QUALIFIED_LEG_MIN_QNV).length;

  // The balance cap applies to the QNV required for the target rank, not to all
  // excess volume. E.g. Diamond requires 500k and max 50% from one leg, so no one
  // leg may contribute more than 250k toward satisfying that 500k qualification.
  let legContributionCap = null;
  let balancedQnv = totalQnv;
  if (rank.maxLegBps !== null) {
    legContributionCap = (rank.qnv * BigInt(rank.maxLegBps)) / BPS_DENOMINATOR;
    balancedQnv = sum(volumes.map((v) => (v > legContributionCap ? legContributionCap : v)));
  }

  const volumePass = totalQnv >= rank.qnv;
  const legsPass = qualifiedLegCount >= rank.qualifiedLegs;
  const balancePass = balancedQnv >= rank.qnv;

  return Object.freeze({
    rankId: rank.id,
    rankName: rank.name,
    requiredQnv: rank.qnv,
    totalQnv,
    qualifiedLegCount,
    requiredQualifiedLegs: rank.qualifiedLegs,
    legContributionCap,
    balancedQnv,
    volumePass,
    legsPass,
    balancePass,
    qualified: volumePass && legsPass && balancePass,
  });
}

export function evaluateCurrentRank(legVolumes) {
  const volumes = legVolumes.map((value, index) =>
    asNonNegativeBigInt(value, `legVolumes[${index}]`),
  );

  const evidence = RANKS.map((rank) => rankEvidenceFor(rank, volumes));
  let current = null;
  for (let i = 0; i < RANKS.length; i += 1) {
    if (evidence[i].qualified) current = RANKS[i];
  }

  return Object.freeze({
    currentRankId: current?.id ?? null,
    currentRankName: current?.name ?? null,
    totalQnv: sum(volumes),
    qualifiedLegCount: volumes.filter((v) => v >= QUALIFIED_LEG_MIN_QNV).length,
    evidence: Object.freeze(evidence),
  });
}

export function updateHighestLifetimeRank(previousRankId, currentRankId) {
  if (previousRankId !== null && !RANK_INDEX.has(previousRankId)) {
    throw new RangeError(`unknown previous rank: ${previousRankId}`);
  }
  if (currentRankId !== null && !RANK_INDEX.has(currentRankId)) {
    throw new RangeError(`unknown current rank: ${currentRankId}`);
  }
  if (previousRankId === null) return currentRankId;
  if (currentRankId === null) return previousRankId;
  return RANK_INDEX.get(currentRankId) > RANK_INDEX.get(previousRankId)
    ? currentRankId
    : previousRankId;
}

export function pioneerBadge(positionCount) {
  const positions = asNonNegativeBigInt(positionCount, "positionCount");
  if (positions > 100n) throw new RangeError("Pioneer position count cannot exceed 100");
  return positions === 0n ? null : `PIONEER ×${positions}`;
}
