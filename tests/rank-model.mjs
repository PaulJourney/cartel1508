import assert from "node:assert/strict";
import {
  QUALIFIED_LEG_MIN_QNV,
  RANKS,
  evaluateCurrentRank,
  pioneerBadge,
  rankEvidenceFor,
  updateHighestLifetimeRank,
} from "../offchain/rank-engine.mjs";

assert.equal(QUALIFIED_LEG_MIN_QNV, 100n);
assert.deepEqual(
  RANKS.map((r) => r.name),
  [
    "Bronze",
    "Silver",
    "Gold",
    "Platinum",
    "Emerald",
    "Diamond",
    "Double Diamond",
    "Crown",
    "Crown Ambassador",
  ],
);

// Personal volume and claimed earnings are deliberately absent from the engine API:
// only rolling 30-day network volume grouped by immutable direct-referral leg enters.
assert.equal(evaluateCurrentRank([500n, 500n]).currentRankId, "bronze");
assert.equal(evaluateCurrentRank([4_900n, 100n]).currentRankId, "silver");

// Gold: 15k QNV, 3 qualified legs and max 60% (=9k) contribution from one leg.
{
  const good = evaluateCurrentRank([9_000n, 5_900n, 100n]);
  assert.equal(good.currentRankId, "gold");
  const gold = good.evidence.find((e) => e.rankId === "gold");
  assert.equal(gold.legContributionCap, 9_000n);
  assert.equal(gold.balancedQnv, 15_000n);
  assert.equal(gold.qualified, true);

  const concentrated = evaluateCurrentRank([14_000n, 500n, 500n]);
  assert.equal(concentrated.currentRankId, "silver");
  const failedGold = concentrated.evidence.find((e) => e.rankId === "gold");
  assert.equal(failedGold.totalQnv, 15_000n);
  assert.equal(failedGold.qualifiedLegCount, 3);
  assert.equal(failedGold.balancePass, false);
}

// A leg below 100 QNV does not count as a Qualified Leg even when total volume passes.
{
  const gold = RANKS.find((r) => r.id === "gold");
  const evidence = rankEvidenceFor(gold, [9_000n, 5_950n, 50n]);
  assert.equal(evidence.totalQnv, 15_000n);
  assert.equal(evidence.qualifiedLegCount, 2);
  assert.equal(evidence.legsPass, false);
  assert.equal(evidence.qualified, false);
}

// Exact Diamond boundary with six real legs and the 50% balance cap.
{
  const result = evaluateCurrentRank([
    250_000n,
    100_000n,
    75_000n,
    50_000n,
    15_000n,
    10_000n,
  ]);
  assert.equal(result.totalQnv, 500_000n);
  assert.equal(result.currentRankId, "diamond");
}

// A whale leg cannot manufacture Diamond: excess above the 250k contribution cap
// is ignored for Diamond qualification even though gross QNV exceeds 500k.
{
  const result = evaluateCurrentRank([
    490_000n,
    2_000n,
    2_000n,
    2_000n,
    2_000n,
    2_000n,
  ]);
  assert.equal(result.totalQnv, 500_000n);
  const diamond = result.evidence.find((e) => e.rankId === "diamond");
  assert.equal(diamond.qualifiedLegCount, 6);
  assert.equal(diamond.balancedQnv, 260_000n);
  assert.equal(diamond.balancePass, false);
  assert.notEqual(result.currentRankId, "diamond");
}

// Highest Lifetime Rank never decreases when Current Rank falls later.
assert.equal(updateHighestLifetimeRank(null, "gold"), "gold");
assert.equal(updateHighestLifetimeRank("gold", "silver"), "gold");
assert.equal(updateHighestLifetimeRank("gold", "diamond"), "diamond");
assert.equal(updateHighestLifetimeRank("diamond", null), "diamond");

// Pioneer is a separate permanent badge and can represent multiple positions.
assert.equal(pioneerBadge(0), null);
assert.equal(pioneerBadge(1), "PIONEER ×1");
assert.equal(pioneerBadge(7), "PIONEER ×7");
assert.equal(pioneerBadge(100), "PIONEER ×100");
assert.throws(() => pioneerBadge(101), /cannot exceed 100/);

console.log("rank model: QNV / qualified legs / balance caps / Pioneer badge passed");
