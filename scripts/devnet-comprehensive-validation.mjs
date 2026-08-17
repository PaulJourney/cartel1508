import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { BN, Program } from "@anchor-lang/core";
import {
  Connection,
  Keypair,
  LAMPORTS_PER_SOL,
  PublicKey,
  SystemProgram,
  Transaction,
  sendAndConfirmTransaction,
} from "@solana/web3.js";
import {
  TOKEN_PROGRAM_ID,
  createMint,
  getAccount,
  getMint,
  getOrCreateAssociatedTokenAccount,
  mintTo,
} from "@solana/spl-token";

const RPC_URL = process.env.ANCHOR_PROVIDER_URL || "https://api.devnet.solana.com";
const IDL_PATH = process.env.PROTOCOL_IDL || "target/idl/service_referral_protocol.json";
const WALLET_PATH = (process.env.ANCHOR_WALLET || "~/.config/solana/id.json").replace(
  /^~(?=$|\/)/,
  os.homedir(),
);
const ZERO_PUBKEY = new PublicKey("11111111111111111111111111111111");
const TOKEN_SCALE = 1_000_000n;
const PIONEER_SCALE = 1_000_000_000_000_000_000n;
const NETWORK_ATOMS = [
  150_000n, 90_000n, 60_000n, 40_000n, 25_000n, 20_000n, 15_000n, 10_000n, 20_000n,
];

function invariant(condition, message) {
  if (!condition) throw new Error(`PRE-MAINNET INVARIANT FAILED: ${message}`);
}

function readKeypair(filename) {
  const bytes = JSON.parse(fs.readFileSync(filename, "utf8"));
  return Keypair.fromSecretKey(Uint8Array.from(bytes));
}

function asBigInt(value) {
  return BigInt(value.toString());
}

function field(state, base, mintKind) {
  const suffix = mintKind === "USDT" ? "Usdt" : "Usdc";
  return asBigInt(state[`${base}${suffix}`]);
}

async function chainUnixTime(connection) {
  const slot = await connection.getSlot("confirmed");
  const t = await connection.getBlockTime(slot);
  return t ?? Math.floor(Date.now() / 1000);
}

async function waitUntilChainTime(connection, unixTime) {
  while ((await chainUnixTime(connection)) < unixTime) {
    await new Promise((resolve) => setTimeout(resolve, 1_000));
  }
}

async function send(connection, signer, instruction, label) {
  const signature = await sendAndConfirmTransaction(
    connection,
    new Transaction().add(instruction),
    [signer],
    { commitment: "confirmed" },
  );
  console.log(`PASS tx | ${label}: ${signature}`);
  return signature;
}

async function expectFailure(connection, signer, instruction, label) {
  try {
    await sendAndConfirmTransaction(
      connection,
      new Transaction().add(instruction),
      [signer],
      { commitment: "confirmed" },
    );
  } catch (error) {
    console.log(`PASS reject | ${label}: ${String(error.message || error).slice(0, 240)}`);
    return;
  }
  throw new Error(`PRE-MAINNET INVARIANT FAILED: expected rejection: ${label}`);
}

const payer = readKeypair(path.resolve(WALLET_PATH));
const treasury = Keypair.generate();
const idl = JSON.parse(fs.readFileSync(path.resolve(IDL_PATH), "utf8"));
const connection = new Connection(RPC_URL, "confirmed");
const program = new Program(idl, { connection });
const programId = new PublicKey(idl.address);

if (process.env.PROGRAM_ID) {
  invariant(
    programId.equals(new PublicKey(process.env.PROGRAM_ID)),
    "IDL Program ID differs from deployed PROGRAM_ID",
  );
}

console.log(`RPC:        ${RPC_URL}`);
console.log(`Program ID: ${programId.toBase58()}`);
console.log(`Deployer:   ${payer.publicKey.toBase58()}`);

const [protocol] = PublicKey.findProgramAddressSync([Buffer.from("protocol")], programId);
const [vaultAuthority] = PublicKey.findProgramAddressSync(
  [Buffer.from("vault-authority")],
  programId,
);
const [technicalRoot] = PublicKey.findProgramAddressSync(
  [Buffer.from("user"), ZERO_PUBKEY.toBuffer()],
  programId,
);

const usdtMint = await createMint(connection, payer, payer.publicKey, null, 6);
const usdcMint = await createMint(connection, payer, payer.publicKey, null, 6);
const unsupportedMint = await createMint(connection, payer, payer.publicKey, null, 6);

const mints = {
  USDT: usdtMint,
  USDC: usdcMint,
  UNSUPPORTED: unsupportedMint,
};

const trackedTokenAccounts = new Map([
  ["USDT", new Set()],
  ["USDC", new Set()],
  ["UNSUPPORTED", new Set()],
]);
const mintedAtoms = new Map([
  ["USDT", 0n],
  ["USDC", 0n],
  ["UNSUPPORTED", 0n],
]);
let totalSuccessfulUnits = 0n;
let registeredUsers = 0n;

function mintKindFor(mint) {
  if (mint.equals(usdtMint)) return "USDT";
  if (mint.equals(usdcMint)) return "USDC";
  return "UNSUPPORTED";
}

function trackAta(kind, address) {
  trackedTokenAccounts.get(kind).add(address.toBase58());
}

async function ata(owner, mint, allowOwnerOffCurve = false) {
  const kind = mintKindFor(mint);
  const account = await getOrCreateAssociatedTokenAccount(
    connection,
    payer,
    mint,
    owner,
    allowOwnerOffCurve,
    "confirmed",
    undefined,
    TOKEN_PROGRAM_ID,
  );
  trackAta(kind, account.address);
  return account.address;
}

async function mintUnits(user, mint, units) {
  const kind = mintKindFor(mint);
  const address = await ata(user.keypair.publicKey, mint, false);
  const amount = BigInt(units) * TOKEN_SCALE;
  await mintTo(connection, payer, mint, address, payer, amount);
  mintedAtoms.set(kind, mintedAtoms.get(kind) + amount);
  return address;
}

const usdtVault = await ata(vaultAuthority, usdtMint, true);
const usdcVault = await ata(vaultAuthority, usdcMint, true);
const treasuryUsdt = await ata(treasury.publicKey, usdtMint, false);
const treasuryUsdc = await ata(treasury.publicKey, usdcMint, false);

const openAt = (await chainUnixTime(connection)) + 4;
await send(
  connection,
  payer,
  await program.methods
    .initialize(new BN(openAt))
    .accounts({
      initializer: payer.publicKey,
      serviceTreasury: treasury.publicKey,
      usdtMint,
      usdcMint,
      protocol,
      vaultAuthority,
      technicalRoot,
      systemProgram: SystemProgram.programId,
    })
    .instruction(),
  "initialize exact final core",
);
await waitUntilChainTime(connection, openAt);

const allUsers = [];
async function fundUserGas(keypair, sol = 0.03) {
  await send(
    connection,
    payer,
    SystemProgram.transfer({
      fromPubkey: payer.publicKey,
      toPubkey: keypair.publicKey,
      lamports: Math.floor(sol * LAMPORTS_PER_SOL),
    }),
    `fund ${keypair.publicKey.toBase58().slice(0, 8)} gas`,
  );
}

async function registerUser(name, parent = null) {
  const keypair = Keypair.generate();
  await fundUserGas(keypair);
  const [pda] = PublicKey.findProgramAddressSync(
    [Buffer.from("user"), keypair.publicKey.toBuffer()],
    programId,
  );
  const referrerWallet = parent ? parent.keypair.publicKey : ZERO_PUBKEY;
  const referrer = parent ? parent.pda : technicalRoot;
  await send(
    connection,
    keypair,
    await program.methods
      .register()
      .accounts({
        wallet: keypair.publicKey,
        protocol,
        referrerWallet,
        referrer,
        user: pda,
        systemProgram: SystemProgram.programId,
      })
      .instruction(),
    `register ${name}`,
  );
  const ancestors = parent
    ? [parent.pda, ...parent.ancestors].slice(0, 11)
    : [];
  const user = { name, keypair, pda, parent, ancestors, purchaseCount: 0n };
  allUsers.push(user);
  registeredUsers += 1n;
  return user;
}

function ancestryAccounts(user) {
  const a = user.ancestors;
  return {
    directReferrer: a[0] || technicalRoot,
    upline1: a[1] || technicalRoot,
    upline2: a[2] || technicalRoot,
    upline3: a[3] || technicalRoot,
    upline4: a[4] || technicalRoot,
    upline5: a[5] || technicalRoot,
    upline6: a[6] || technicalRoot,
    upline7: a[7] || technicalRoot,
    upline8: a[8] || technicalRoot,
  };
}

async function purchaseInstruction(user, mint, source, units, overrides = {}) {
  return program.methods
    .purchaseAndDistribute(new BN(units.toString()))
    .accounts({
      wallet: user.keypair.publicKey,
      protocol,
      user: user.pda,
      userSource: source,
      vaultAuthority,
      usdtVault,
      usdcVault,
      serviceTreasuryUsdt: treasuryUsdt,
      serviceTreasuryUsdc: treasuryUsdc,
      ...ancestryAccounts(user),
      tokenProgram: TOKEN_PROGRAM_ID,
      ...overrides,
    })
    .instruction();
}

async function purchase(user, mint, units, label, source = null, overrides = {}) {
  const actualSource = source || await mintUnits(user, mint, units);
  const ix = await purchaseInstruction(user, mint, actualSource, BigInt(units), overrides);
  const sig = await send(connection, user.keypair, ix, label);
  totalSuccessfulUnits += BigInt(units);
  user.purchaseCount += 1n;
  return { sig, source: actualSource };
}

async function userState(user) {
  return program.account.userState.fetch(user.pda);
}

async function protocolState() {
  return program.account.protocolState.fetch(protocol);
}

async function tokenAmount(address) {
  return (await getAccount(connection, address, "confirmed", TOKEN_PROGRAM_ID)).amount;
}

async function treasuryAmount(kind) {
  return tokenAmount(kind === "USDT" ? treasuryUsdt : treasuryUsdc);
}

async function vaultAmount(kind) {
  return tokenAmount(kind === "USDT" ? usdtVault : usdcVault);
}

function pioneerDueAtoms(state, pstate, kind) {
  const index = asBigInt(kind === "USDT" ? pstate.pioneerIndexUsdt : pstate.pioneerIndexUsdc);
  const checkpoint = asBigInt(
    kind === "USDT" ? state.pioneerCheckpointUsdt : state.pioneerCheckpointUsdc,
  );
  const positions = asBigInt(state.pioneerPositions);
  const scaled = index * positions - checkpoint;
  invariant(scaled >= 0n, `${kind} Pioneer checkpoint exceeds entitlement`);
  return scaled / PIONEER_SCALE;
}

async function claimExact(user, kind, label) {
  const mint = mints[kind];
  const destination = await ata(user.keypair.publicKey, mint, false);
  const stateBefore = await userState(user);
  const pBefore = await protocolState();
  const direct = field(stateBefore, "selfAccrued", kind);
  const network = field(stateBefore, "networkClaimable", kind);
  const pioneer = pioneerDueAtoms(stateBefore, pBefore, kind);
  const expected = direct + network + pioneer;
  invariant(expected > 0n, `${label}: expected positive claim`);
  const balanceBefore = await tokenAmount(destination);
  const lifetimeBefore = field(stateBefore, "lifetimeClaimed", kind);

  await send(
    connection,
    user.keypair,
    await program.methods
      .claim()
      .accounts({
        wallet: user.keypair.publicKey,
        protocol,
        user: user.pda,
        vaultAuthority,
        vaultToken: kind === "USDT" ? usdtVault : usdcVault,
        destination,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .instruction(),
    label,
  );

  const stateAfter = await userState(user);
  const balanceAfter = await tokenAmount(destination);
  invariant(balanceAfter - balanceBefore === expected, `${label}: token payout mismatch`);
  invariant(field(stateAfter, "selfAccrued", kind) === 0n, `${label}: SELF not cleared`);
  invariant(field(stateAfter, "networkClaimable", kind) === 0n, `${label}: network not cleared`);
  invariant(
    field(stateAfter, "lifetimeClaimed", kind) - lifetimeBefore === expected,
    `${label}: lifetime claimed mismatch`,
  );
  return { expected, direct, network, pioneer };
}

async function claimIfAny(user, kind) {
  const state = await userState(user);
  const pstate = await protocolState();
  const expected =
    field(state, "selfAccrued", kind) +
    field(state, "networkClaimable", kind) +
    pioneerDueAtoms(state, pstate, kind);
  if (expected === 0n) return 0n;
  const result = await claimExact(user, kind, `final sweep ${user.name} ${kind}`);
  return result.expected;
}

async function assertConservation(kind) {
  let sum = 0n;
  for (const address of trackedTokenAccounts.get(kind)) {
    sum += await tokenAmount(new PublicKey(address));
  }
  const supply = (await getMint(connection, mints[kind], "confirmed", TOKEN_PROGRAM_ID)).supply;
  invariant(sum === supply, `${kind} known-token-account sum must equal mint supply`);
  invariant(sum === mintedAtoms.get(kind), `${kind} mint supply must equal tracked minted amount`);
}

console.log("\n=== PHASE 1: full genealogy + exact network-depth boundaries (real Devnet USDC) ===");

const chain = [];
let parent = null;
for (let i = 0; i < 12; i += 1) {
  const user = await registerUser(i === 11 ? "buyer" : `ancestor-${i}`, parent);
  chain.push(user);
  parent = user;
}
const buyer = chain[11];

const preBoundaryUnits = new Map([
  [10, 10],
  [9, 10],
  [8, 10],
  [7, 24],
  [6, 49],
  [5, 99],
  [4, 199],
  [3, 349],
  [2, 499],
]);

for (const [index, units] of [...preBoundaryUnits.entries()].sort((a, b) => a[0] - b[0])) {
  await purchase(chain[index], usdcMint, units, `${chain[index].name} personal ${units} USDC`);
}

const buyerUsdc = await mintUnits(buyer, usdcMint, 200);
const beforeLocked = await Promise.all(chain.map(userState));
const treasuryBeforeLocked = await treasuryAmount("USDC");
await purchase(buyer, usdcMint, 100, "buyer target purchase: below-boundary U4-U9", buyerUsdc);
const afterLocked = await Promise.all(chain.map(userState));
const treasuryAfterLocked = await treasuryAmount("USDC");

for (let level = 1; level <= 9; level += 1) {
  const index = 11 - level;
  const delta =
    field(afterLocked[index], "networkClaimable", "USDC") -
    field(beforeLocked[index], "networkClaimable", "USDC");
  const expected = level <= 3 ? NETWORK_ATOMS[level - 1] * 100n : 0n;
  invariant(delta === expected, `locked-depth target U${level} delta mismatch`);
}
for (const index of [0, 1]) {
  invariant(
    field(afterLocked[index], "networkClaimable", "USDC") ===
      field(beforeLocked[index], "networkClaimable", "USDC"),
    `U${11 - index} outside nine-level cap changed`,
  );
}
invariant(
  treasuryAfterLocked - treasuryBeforeLocked === 20n * TOKEN_SCALE,
  "below-boundary purchase must route exactly 20% to Treasury (5 service + 13 locked + 2 Pioneer-unassigned)",
);

for (const index of [7, 6, 5, 4, 3, 2]) {
  await purchase(chain[index], usdcMint, 1, `${chain[index].name} crosses exact depth boundary +1`);
  const state = await userState(chain[index]);
  const expectedUnits = BigInt(preBoundaryUnits.get(index) + 1);
  invariant(asBigInt(state.currentWeekUnits) === expectedUnits, `${chain[index].name} weekly units boundary mismatch`);
}

const beforeFull = await Promise.all(chain.map(userState));
const treasuryBeforeFull = await treasuryAmount("USDC");
await purchase(buyer, usdcMint, 100, "buyer target purchase: all U1-U9 exactly qualified", buyerUsdc);
const afterFull = await Promise.all(chain.map(userState));
const treasuryAfterFull = await treasuryAmount("USDC");

for (let level = 1; level <= 9; level += 1) {
  const index = 11 - level;
  const delta =
    field(afterFull[index], "networkClaimable", "USDC") -
    field(beforeFull[index], "networkClaimable", "USDC");
  const expected = NETWORK_ATOMS[level - 1] * 100n;
  invariant(delta === expected, `full-depth target U${level} delta mismatch`);
}
for (const index of [0, 1]) {
  invariant(
    field(afterFull[index], "networkClaimable", "USDC") ===
      field(beforeFull[index], "networkClaimable", "USDC"),
    `out-of-range ancestor index ${index} changed on full-depth target`,
  );
}
invariant(
  treasuryAfterFull - treasuryBeforeFull === 7n * TOKEN_SCALE,
  "full-depth purchase must route exactly 7% to Treasury before any Pioneer positions exist",
);

const buyerStateAfterTargets = await userState(buyer);
invariant(asBigInt(buyerStateAfterTargets.currentWeekUnits) === 200n, "buyer weekly units must accumulate to 200 while ACTIVE");
invariant(asBigInt(buyerStateAfterTargets.activeWeeksStarted) === 1n, "buyer active-week counter must not advance on second ACTIVE purchase");
invariant(asBigInt(buyerStateAfterTargets.nextPurchaseIndex) === 2n, "buyer purchase index must be 2 after two target purchases");

console.log("\n=== PHASE 2: batching equivalence + dual-token separation ===");

const batchSingle = await registerUser("batch-single");
const batchTen = await registerUser("batch-ten");
const singleUsdc = await mintUnits(batchSingle, usdcMint, 10);
const tenUsdc = await mintUnits(batchTen, usdcMint, 10);
await purchase(batchSingle, usdcMint, 10, "batch-single 1x10", singleUsdc);
for (let i = 0; i < 10; i += 1) {
  await purchase(batchTen, usdcMint, 1, `batch-ten ${i + 1}/10`, tenUsdc);
}
const singleState = await userState(batchSingle);
const tenState = await userState(batchTen);
invariant(field(singleState, "selfAccrued", "USDC") === 5n * TOKEN_SCALE, "1x10 SELF mismatch");
invariant(field(tenState, "selfAccrued", "USDC") === 5n * TOKEN_SCALE, "10x1 SELF mismatch");
invariant(asBigInt(singleState.currentWeekUnits) === 10n, "1x10 current week units mismatch");
invariant(asBigInt(tenState.currentWeekUnits) === 10n, "10x1 current week units mismatch");
invariant(asBigInt(singleState.activeWeeksStarted) === 1n, "1x10 must start one ACTIVE week");
invariant(asBigInt(tenState.activeWeeksStarted) === 1n, "10x1 must start one ACTIVE week");

const dual = await registerUser("dual-token");
const dualUsdt = await mintUnits(dual, usdtMint, 10);
const dualUsdc = await mintUnits(dual, usdcMint, 10);
await purchase(dual, usdtMint, 10, "dual-token 10 USDT", dualUsdt);
await purchase(dual, usdcMint, 10, "dual-token 10 USDC while ACTIVE", dualUsdc);
let dualState = await userState(dual);
invariant(field(dualState, "selfAccrued", "USDT") === 5n * TOKEN_SCALE, "dual USDT SELF mismatch");
invariant(field(dualState, "selfAccrued", "USDC") === 5n * TOKEN_SCALE, "dual USDC SELF mismatch");
invariant(asBigInt(dualState.currentWeekUnits) === 20n, "dual-token purchases must share personal weekly units");

const dualUsdcBeforeUsdtClaim = field(dualState, "selfAccrued", "USDC");
await claimExact(dual, "USDT", "dual-token claims USDT only");
dualState = await userState(dual);
invariant(
  field(dualState, "selfAccrued", "USDC") === dualUsdcBeforeUsdtClaim,
  "USDT claim must not mutate USDC accrual",
);
await claimExact(dual, "USDC", "dual-token claims USDC only");

console.log("\n=== PHASE 3: adversarial / rollback transactions on real Devnet ===");

// Keep spendable USDC available so tampered-ancestry rejection cannot be a false
// positive caused by insufficient token balance before the ancestry walk executes.
await mintUnits(buyer, usdcMint, 10);
const buyerUsdcBalanceBeforeBad = await tokenAmount(buyerUsdc);
const pBeforeBad = await protocolState();
const statesBeforeBad = await Promise.all(chain.map(userState));

await expectFailure(
  connection,
  buyer.keypair,
  await purchaseInstruction(buyer, usdcMint, buyerUsdc, 0n),
  "zero-unit purchase",
);

const tooLargeUnits = 18_446_744_073_710n;
await expectFailure(
  connection,
  buyer.keypair,
  await purchaseInstruction(buyer, usdcMint, buyerUsdc, tooLargeUnits),
  "purchase amount exceeding u64 token atoms",
);

await expectFailure(
  connection,
  buyer.keypair,
  await purchaseInstruction(buyer, usdcMint, buyerUsdc, 1n, {
    upline1: chain[10].pda,
  }),
  "tampered upline ancestry",
);

await expectFailure(
  connection,
  buyer.keypair,
  await purchaseInstruction(buyer, usdcMint, buyerUsdc, 1n, {
    usdcVault: buyerUsdc,
  }),
  "wrong canonical USDC vault",
);

const unsupportedSource = await mintUnits(buyer, unsupportedMint, 10);
await expectFailure(
  connection,
  buyer.keypair,
  await purchaseInstruction(buyer, unsupportedMint, unsupportedSource, 10n),
  "unsupported mint",
);

const foreignSource = await mintUnits(chain[10], usdcMint, 1);
await expectFailure(
  connection,
  buyer.keypair,
  await purchaseInstruction(buyer, usdcMint, foreignSource, 1n),
  "wrong token authority",
);

const buyerUsdcBalanceAfterBad = await tokenAmount(buyerUsdc);
const pAfterBad = await protocolState();
const statesAfterBad = await Promise.all(chain.map(userState));
invariant(buyerUsdcBalanceAfterBad === buyerUsdcBalanceBeforeBad, "failed buyer transactions moved buyer USDC");
invariant(asBigInt(pAfterBad.nextUnitId) === asBigInt(pBeforeBad.nextUnitId), "failed transactions advanced global Unit ID");
for (let i = 0; i < chain.length; i += 1) {
  invariant(
    field(statesAfterBad[i], "networkClaimable", "USDC") ===
      field(statesBeforeBad[i], "networkClaimable", "USDC"),
    `failed ancestry tests mutated network state for chain index ${i}`,
  );
}

console.log("\n=== PHASE 4: Pioneer purchase provenance, weighted checkpoints, Rule B, 98->100 cap ===");

const split500 = await registerUser("pioneer-500-plus-500");
const splitSource = await mintUnits(split500, usdtMint, 1000);
await purchase(split500, usdtMint, 500, "Pioneer non-cumulative purchase 500 #1", splitSource);
await purchase(split500, usdtMint, 500, "Pioneer non-cumulative purchase 500 #2", splitSource);
let splitState = await userState(split500);
invariant(asBigInt(splitState.pioneerPositions) === 0n, "500+500 separate purchases must create zero Pioneer positions");

const pioneerBaseline = await protocolState();
const pioneerUnassignedBaseline = asBigInt(pioneerBaseline.lifetimePioneerUnassignedUsdt);

const p1 = await registerUser("pioneer-one");
const p2 = await registerUser("pioneer-two");
const p3 = await registerUser("pioneer-saturator");
const trigger = await registerUser("pioneer-trigger");

const p1Source = await mintUnits(p1, usdtMint, 2000);
const p2Source = await mintUnits(p2, usdtMint, 2000);
const p3Source = await mintUnits(p3, usdtMint, 102_000);
const triggerSource = await mintUnits(trigger, usdtMint, 100);

await purchase(p1, usdtMint, 1000, "P1 buys 1000 -> 1 position after own event", p1Source);
let p1State = await userState(p1);
let ps = await protocolState();
invariant(asBigInt(p1State.pioneerPositions) === 1n, "P1 must own exactly 1 Pioneer position");
invariant(pioneerDueAtoms(p1State, ps, "USDT") === 0n, "Rule B: P1 must not earn on its creating purchase");

await purchase(p2, usdtMint, 2000, "P2 buys 2000 -> 2 positions after P1 earns", p2Source);
p1State = await userState(p1);
let p2State = await userState(p2);
ps = await protocolState();
invariant(asBigInt(p2State.pioneerPositions) === 2n, "P2 must own exactly 2 Pioneer positions");
invariant(pioneerDueAtoms(p1State, ps, "USDT") === 400_000n, "P1 due after P2 purchase must be 0.4 USDT");
invariant(pioneerDueAtoms(p2State, ps, "USDT") === 0n, "Rule B: P2 must not earn on its creating purchase");

await purchase(p1, usdtMint, 1000, "P1 buys another 1000 -> adds 1 at weighted checkpoint", p1Source);
p1State = await userState(p1);
p2State = await userState(p2);
ps = await protocolState();
invariant(asBigInt(p1State.pioneerPositions) === 2n, "P1 must now own exactly 2 positions");
invariant(pioneerDueAtoms(p1State, ps, "USDT") === 600_000n, "P1 weighted due before trigger must be 0.6 USDT");
invariant(pioneerDueAtoms(p2State, ps, "USDT") === 400_000n, "P2 due before trigger must be 0.4 USDT");

await purchase(trigger, usdtMint, 100, "100-unit global trigger with 4 assigned Pioneer positions", triggerSource);
p1State = await userState(p1);
p2State = await userState(p2);
ps = await protocolState();
invariant(pioneerDueAtoms(p1State, ps, "USDT") === 640_000n, "P1 weighted Pioneer due must be 0.64 USDT");
invariant(pioneerDueAtoms(p2State, ps, "USDT") === 440_000n, "P2 weighted Pioneer due must be 0.44 USDT");
invariant(asBigInt(ps.pioneerPositionsAssigned) === 4n, "global Pioneer positions must be 4 before saturation");

const p1Claim1 = await claimExact(p1, "USDT", "P1 claims SELF + weighted Pioneer");
const p2Claim1 = await claimExact(p2, "USDT", "P2 claims SELF + weighted Pioneer");
invariant(p1Claim1.pioneer === 640_000n, "P1 first Pioneer claim exact due mismatch");
invariant(p2Claim1.pioneer === 440_000n, "P2 first Pioneer claim exact due mismatch");

await purchase(p3, usdtMint, 94_000, "P3 buys 94000 -> global 4 to 98 positions", p3Source);
ps = await protocolState();
let p3State = await userState(p3);
invariant(asBigInt(ps.pioneerPositionsAssigned) === 98n, "global Pioneer pool must reach 98");
invariant(asBigInt(p3State.pioneerPositions) === 94n, "P3 must receive exactly 94 positions");

await purchase(p3, usdtMint, 3_000, "P3 buys 3000 at 98/100 -> receives exactly final 2", p3Source);
ps = await protocolState();
p3State = await userState(p3);
invariant(asBigInt(ps.pioneerPositionsAssigned) === 100n, "98 + capped purchase must reach exactly 100");
invariant(asBigInt(p3State.pioneerPositions) === 96n, "P3 must own 96 after cap truncates 3 candidates to 2");

const unassignedBeforeSaturatedPurchase = asBigInt(ps.lifetimePioneerUnassignedUsdt);
await purchase(p3, usdtMint, 5_000, "P3 buys 5000 after 100/100 -> zero new positions", p3Source);
ps = await protocolState();
p3State = await userState(p3);
invariant(asBigInt(ps.pioneerPositionsAssigned) === 100n, "Pioneer pool must stay at 100 forever");
invariant(asBigInt(p3State.pioneerPositions) === 96n, "post-saturation purchase must add zero P3 positions");
invariant(
  asBigInt(ps.lifetimePioneerUnassignedUsdt) === unassignedBeforeSaturatedPurchase,
  "fully assigned Pioneer pool must create zero new unassigned Pioneer flow",
);

const pioneerUnassignedDelta =
  asBigInt(ps.lifetimePioneerUnassignedUsdt) - pioneerUnassignedBaseline;
invariant(
  pioneerUnassignedDelta === 1_886_920_000n,
  `Pioneer unassigned audit delta mismatch: ${pioneerUnassignedDelta}`,
);

const p3Claim = await claimExact(p3, "USDT", "P3 claims SELF + Pioneer after saturation");
invariant(p3Claim.pioneer === 152_400_000n, "P3 Pioneer due must be exactly 152.4 USDT");

const p1Claim2 = await claimExact(p1, "USDT", "P1 claims post-saturation Pioneer");
const p2Claim2 = await claimExact(p2, "USDT", "P2 claims post-saturation Pioneer");
invariant(p1Claim2.pioneer === 40_800_000n, "P1 post-saturation Pioneer due must be 40.8 USDT");
invariant(p2Claim2.pioneer === 40_800_000n, "P2 post-saturation Pioneer due must be 40.8 USDT");

console.log("\n=== PHASE 5: pull-claims, final liability sweep, global conservation ===");

for (const user of allUsers) {
  const state = await userState(user);
  if (asBigInt(state.activeUntil) > 0n) {
    await claimIfAny(user, "USDC");
    await claimIfAny(user, "USDT");
  }
}

invariant(await vaultAmount("USDC") === 0n, "final USDC vault must be zero after complete active claim sweep");
invariant(await vaultAmount("USDT") === 0n, "final USDT vault must be zero after complete active claim sweep");

const finalProtocol = await protocolState();
invariant(asBigInt(finalProtocol.realUserCount) === registeredUsers, "protocol real_user_count mismatch");
invariant(
  asBigInt(finalProtocol.nextUnitId) === totalSuccessfulUnits + 1n,
  "global Unit IDs must equal exact sum of successful purchases + 1",
);
invariant(asBigInt(finalProtocol.pioneerPositionsAssigned) === 100n, "final Pioneer global count must be exactly 100");

for (const user of allUsers) {
  const state = await userState(user);
  invariant(
    asBigInt(state.nextPurchaseIndex) === user.purchaseCount,
    `${user.name} next_purchase_index mismatch`,
  );
}

await assertConservation("USDC");
await assertConservation("USDT");
await assertConservation("UNSUPPORTED");
invariant(
  await tokenAmount(unsupportedSource) === 10n * TOKEN_SCALE,
  "unsupported-mint rejection must leave unsupported source untouched",
);

console.log("DEVNET PRE-MAINNET COMPREHENSIVE VALIDATION: PASS");
console.log(JSON.stringify({
  programId: programId.toBase58(),
  registeredUsers: registeredUsers.toString(),
  successfulUnits: totalSuccessfulUnits.toString(),
  pioneerPositionsAssigned: asBigInt(finalProtocol.pioneerPositionsAssigned).toString(),
  nextUnitId: asBigInt(finalProtocol.nextUnitId).toString(),
  usdtVault: (await vaultAmount("USDT")).toString(),
  usdcVault: (await vaultAmount("USDC")).toString(),
  usdtSupply: (await getMint(connection, usdtMint, "confirmed", TOKEN_PROGRAM_ID)).supply.toString(),
  usdcSupply: (await getMint(connection, usdcMint, "confirmed", TOKEN_PROGRAM_ID)).supply.toString(),
}, null, 2));
