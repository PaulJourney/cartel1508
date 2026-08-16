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

function invariant(condition, message) {
  if (!condition) throw new Error(`SMOKE INVARIANT FAILED: ${message}`);
}

function readKeypair(filename) {
  const bytes = JSON.parse(fs.readFileSync(filename, "utf8"));
  return Keypair.fromSecretKey(Uint8Array.from(bytes));
}

function asBigInt(value) {
  return BigInt(value.toString());
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
  console.log(`${label}: ${signature}`);
  return signature;
}

const payer = readKeypair(path.resolve(WALLET_PATH));
const sponsor = Keypair.generate();
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
console.log(`Buyer:      ${payer.publicKey.toBase58()}`);
console.log(`Sponsor:    ${sponsor.publicKey.toBase58()}`);

await send(
  connection,
  payer,
  SystemProgram.transfer({
    fromPubkey: payer.publicKey,
    toPubkey: sponsor.publicKey,
    lamports: Math.floor(0.05 * LAMPORTS_PER_SOL),
  }),
  "fund sponsor devnet gas",
);

const [protocol] = PublicKey.findProgramAddressSync([Buffer.from("protocol")], programId);
const [vaultAuthority] = PublicKey.findProgramAddressSync(
  [Buffer.from("vault-authority")],
  programId,
);
const [technicalRoot] = PublicKey.findProgramAddressSync(
  [Buffer.from("user"), ZERO_PUBKEY.toBuffer()],
  programId,
);
const [sponsorUser] = PublicKey.findProgramAddressSync(
  [Buffer.from("user"), sponsor.publicKey.toBuffer()],
  programId,
);
const [buyerUser] = PublicKey.findProgramAddressSync(
  [Buffer.from("user"), payer.publicKey.toBuffer()],
  programId,
);

const usdtMint = await createMint(connection, payer, payer.publicKey, null, 6);
const usdcMint = await createMint(connection, payer, payer.publicKey, null, 6);

const sponsorUsdt = await getOrCreateAssociatedTokenAccount(
  connection, payer, usdtMint, sponsor.publicKey, false, "confirmed", undefined, TOKEN_PROGRAM_ID,
);
const buyerUsdt = await getOrCreateAssociatedTokenAccount(
  connection, payer, usdtMint, payer.publicKey, false, "confirmed", undefined, TOKEN_PROGRAM_ID,
);
const usdtVault = await getOrCreateAssociatedTokenAccount(
  connection, payer, usdtMint, vaultAuthority, true, "confirmed", undefined, TOKEN_PROGRAM_ID,
);
const usdcVault = await getOrCreateAssociatedTokenAccount(
  connection, payer, usdcMint, vaultAuthority, true, "confirmed", undefined, TOKEN_PROGRAM_ID,
);
const treasuryUsdt = await getOrCreateAssociatedTokenAccount(
  connection, payer, usdtMint, treasury.publicKey, false, "confirmed", undefined, TOKEN_PROGRAM_ID,
);
const treasuryUsdc = await getOrCreateAssociatedTokenAccount(
  connection, payer, usdcMint, treasury.publicKey, false, "confirmed", undefined, TOKEN_PROGRAM_ID,
);

await mintTo(connection, payer, usdtMint, sponsorUsdt.address, payer, 10n * TOKEN_SCALE);
await mintTo(connection, payer, usdtMint, buyerUsdt.address, payer, 100n * TOKEN_SCALE);

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
  "initialize final core",
);
await waitUntilChainTime(connection, openAt);

await send(
  connection,
  sponsor,
  await program.methods
    .register()
    .accounts({
      wallet: sponsor.publicKey,
      protocol,
      referrerWallet: ZERO_PUBKEY,
      referrer: technicalRoot,
      user: sponsorUser,
      systemProgram: SystemProgram.programId,
    })
    .instruction(),
  "register sponsor / no Pioneer position",
);

const rootEight = Object.fromEntries(
  Array.from({ length: 8 }, (_, i) => [`upline${i + 1}`, technicalRoot]),
);

async function buyerPurchase(units, label) {
  const ix = await program.methods
    .purchaseAndDistribute(new BN(units))
    .accounts({
      wallet: payer.publicKey,
      protocol,
      user: buyerUser,
      userSource: buyerUsdt.address,
      vaultAuthority,
      usdtVault: usdtVault.address,
      usdcVault: usdcVault.address,
      serviceTreasuryUsdt: treasuryUsdt.address,
      serviceTreasuryUsdc: treasuryUsdc.address,
      directReferrer: sponsorUser,
      upline1: technicalRoot,
      upline2: technicalRoot,
      upline3: technicalRoot,
      upline4: technicalRoot,
      upline5: technicalRoot,
      upline6: technicalRoot,
      upline7: technicalRoot,
      upline8: technicalRoot,
      tokenProgram: TOKEN_PROGRAM_ID,
    })
    .instruction();
  await send(connection, payer, ix, label);
}

const sponsorPurchaseIx = await program.methods
  .purchaseAndDistribute(new BN(10))
  .accounts({
    wallet: sponsor.publicKey,
    protocol,
    user: sponsorUser,
    userSource: sponsorUsdt.address,
    vaultAuthority,
    usdtVault: usdtVault.address,
    usdcVault: usdcVault.address,
    serviceTreasuryUsdt: treasuryUsdt.address,
    serviceTreasuryUsdc: treasuryUsdc.address,
    directReferrer: technicalRoot,
    ...rootEight,
    tokenProgram: TOKEN_PROGRAM_ID,
  })
  .instruction();
await send(connection, sponsor, sponsorPurchaseIx, "sponsor buys 10 units / SELF activates");

let sponsorToken = await getAccount(connection, sponsorUsdt.address, "confirmed", TOKEN_PROGRAM_ID);
let buyerToken = await getAccount(connection, buyerUsdt.address, "confirmed", TOKEN_PROGRAM_ID);
let treasuryToken = await getAccount(connection, treasuryUsdt.address, "confirmed", TOKEN_PROGRAM_ID);
let vaultToken = await getAccount(connection, usdtVault.address, "confirmed", TOKEN_PROGRAM_ID);

invariant(sponsorToken.amount === 0n, "sponsor activation must spend exactly 10 USDT");
invariant(buyerToken.amount === 100n * TOKEN_SCALE, "buyer funds must remain untouched before buyer purchase");
invariant(treasuryToken.amount === 5_000_000n, "sponsor activation treasury amount mismatch");
invariant(vaultToken.amount === 5_000_000n, "sponsor SELF liability mismatch");

await send(
  connection,
  payer,
  await program.methods
    .register()
    .accounts({
      wallet: payer.publicKey,
      protocol,
      referrerWallet: sponsor.publicKey,
      referrer: sponsorUser,
      user: buyerUser,
      systemProgram: SystemProgram.programId,
    })
    .instruction(),
  "register buyer under sponsor / no Pioneer position",
);

await buyerPurchase(100, "buyer buys 100 units / SELF + nine-upline accounting");

sponsorToken = await getAccount(connection, sponsorUsdt.address, "confirmed", TOKEN_PROGRAM_ID);
buyerToken = await getAccount(connection, buyerUsdt.address, "confirmed", TOKEN_PROGRAM_ID);
treasuryToken = await getAccount(connection, treasuryUsdt.address, "confirmed", TOKEN_PROGRAM_ID);
vaultToken = await getAccount(connection, usdtVault.address, "confirmed", TOKEN_PROGRAM_ID);

invariant(buyerToken.amount === 0n, "buyer purchase must spend exactly 100 USDT");
invariant(treasuryToken.amount === 40_000_000n, "combined treasury balance mismatch after buyer purchase");
invariant(vaultToken.amount === 70_000_000n, "combined vault liabilities mismatch after buyer purchase");

let sponsorState = await program.account.userState.fetch(sponsorUser);
let buyerState = await program.account.userState.fetch(buyerUser);
let protocolState = await program.account.protocolState.fetch(protocol);

invariant(asBigInt(sponsorState.pioneerPositions) === 0n, "10-unit sponsor purchase must not create Pioneer positions");
invariant(asBigInt(buyerState.pioneerPositions) === 0n, "100-unit buyer purchase must not create Pioneer positions");
invariant(asBigInt(protocolState.pioneerPositionsAssigned) === 0n, "registration and sub-1000 purchases must leave Pioneer pool empty");
invariant(asBigInt(sponsorState.selfAccruedUsdt) === 5n * TOKEN_SCALE, "sponsor own SELF reward must be exactly 5 USDT");
invariant(asBigInt(sponsorState.networkClaimableUsdt) === 15n * TOKEN_SCALE, "sponsor U1 reward must be exactly 15 USDT");
invariant(asBigInt(buyerState.selfAccruedUsdt) === 50n * TOKEN_SCALE, "buyer SELF reward must be exactly 50 USDT");
invariant(asBigInt(protocolState.nextUnitId) === 111n, "next global Unit ID must be 111");

await send(
  connection,
  sponsor,
  await program.methods
    .claim()
    .accounts({
      wallet: sponsor.publicKey,
      protocol,
      user: sponsorUser,
      vaultAuthority,
      vaultToken: usdtVault.address,
      destination: sponsorUsdt.address,
      tokenProgram: TOKEN_PROGRAM_ID,
    })
    .instruction(),
  "sponsor pull-claims SELF + U1",
);
await send(
  connection,
  payer,
  await program.methods
    .claim()
    .accounts({
      wallet: payer.publicKey,
      protocol,
      user: buyerUser,
      vaultAuthority,
      vaultToken: usdtVault.address,
      destination: buyerUsdt.address,
      tokenProgram: TOKEN_PROGRAM_ID,
    })
    .instruction(),
  "buyer pull-claims SELF",
);

sponsorToken = await getAccount(connection, sponsorUsdt.address, "confirmed", TOKEN_PROGRAM_ID);
buyerToken = await getAccount(connection, buyerUsdt.address, "confirmed", TOKEN_PROGRAM_ID);
treasuryToken = await getAccount(connection, treasuryUsdt.address, "confirmed", TOKEN_PROGRAM_ID);
vaultToken = await getAccount(connection, usdtVault.address, "confirmed", TOKEN_PROGRAM_ID);

invariant(sponsorToken.amount === 20_000_000n, "initial sponsor claim must pay exactly 20 USDT");
invariant(buyerToken.amount === 50_000_000n, "initial buyer claim must pay exactly 50 USDT");
invariant(treasuryToken.amount === 40_000_000n, "initial claims must not change treasury");
invariant(vaultToken.amount === 0n, "initial liabilities must clear the vault");

// Final Pioneer proof on real devnet transactions. A 98,000-unit single purchase
// creates 98 positions only after its own 2% has been processed (Rule B). The next
// 3,000-unit purchase is capped at the final 2 positions. A later 5,000-unit purchase
// receives zero new positions because 100/100 is an absolute permanent saturation.
await mintTo(connection, payer, usdtMint, buyerUsdt.address, payer, 106_000n * TOKEN_SCALE);

await buyerPurchase(98_000, "buyer buys 98000 / acquires 98 Pioneer positions after current event");
buyerState = await program.account.userState.fetch(buyerUser);
protocolState = await program.account.protocolState.fetch(protocol);
treasuryToken = await getAccount(connection, treasuryUsdt.address, "confirmed", TOKEN_PROGRAM_ID);
vaultToken = await getAccount(connection, usdtVault.address, "confirmed", TOKEN_PROGRAM_ID);
invariant(asBigInt(buyerState.pioneerPositions) === 98n, "98000 single purchase must create exactly 98 positions");
invariant(asBigInt(protocolState.pioneerPositionsAssigned) === 98n, "global Pioneer count must be 98");
invariant(treasuryToken.amount === 34_340_000_000n, "98000 creating purchase must route its entire Pioneer 2% as unassigned under Rule B");
invariant(vaultToken.amount === 63_700_000_000n, "98000 creating purchase must exclude its new 98 positions from current-event Pioneer liability");

await buyerPurchase(3_000, "buyer buys 3000 at 98/100 / receives only final 2 positions");
buyerState = await program.account.userState.fetch(buyerUser);
protocolState = await program.account.protocolState.fetch(protocol);
treasuryToken = await getAccount(connection, treasuryUsdt.address, "confirmed", TOKEN_PROGRAM_ID);
vaultToken = await getAccount(connection, usdtVault.address, "confirmed", TOKEN_PROGRAM_ID);
invariant(asBigInt(buyerState.pioneerPositions) === 100n, "98/100 plus 3000 must cap wallet at exactly 100 positions");
invariant(asBigInt(protocolState.pioneerPositionsAssigned) === 100n, "98/100 plus 3000 must saturate global pool at 100");
invariant(treasuryToken.amount === 35_331_200_000n, "3000 purchase must leave exactly two current-event Pioneer slots unassigned");
invariant(vaultToken.amount === 65_708_800_000n, "only the pre-existing 98 positions may earn on the 3000 creating purchase");

await buyerPurchase(5_000, "buyer buys 5000 after 100/100 / no further Pioneer positions");
buyerState = await program.account.userState.fetch(buyerUser);
protocolState = await program.account.protocolState.fetch(protocol);
treasuryToken = await getAccount(connection, treasuryUsdt.address, "confirmed", TOKEN_PROGRAM_ID);
vaultToken = await getAccount(connection, usdtVault.address, "confirmed", TOKEN_PROGRAM_ID);
invariant(asBigInt(buyerState.pioneerPositions) === 100n, "post-saturation 5000 purchase must create zero additional positions");
invariant(asBigInt(protocolState.pioneerPositionsAssigned) === 100n, "global Pioneer pool must remain permanently capped at 100");
invariant(treasuryToken.amount === 36_981_200_000n, "fully assigned Pioneer pool must have zero unassigned Pioneer flow on later purchase");
invariant(vaultToken.amount === 69_058_800_000n, "full-pool purchase liability mismatch");
invariant(asBigInt(protocolState.nextUnitId) === 106_111n, "global Unit IDs must remain contiguous through Pioneer saturation sequence");
invariant(asBigInt(protocolState.lifetimeServiceFeesUsdt) === 5_305_500_000n, "lifetime service metric mismatch after Pioneer sequence");
invariant(asBigInt(protocolState.lifetimeUnallocatedUsdt) === 29_712_300_000n, "lifetime unallocated metric mismatch after Pioneer sequence");
invariant(asBigInt(protocolState.lifetimePioneerUnassignedUsdt) === 1_963_400_000n, "lifetime Pioneer-unassigned metric mismatch after saturation");

await send(
  connection,
  payer,
  await program.methods
    .claim()
    .accounts({
      wallet: payer.publicKey,
      protocol,
      user: buyerUser,
      vaultAuthority,
      vaultToken: usdtVault.address,
      destination: buyerUsdt.address,
      tokenProgram: TOKEN_PROGRAM_ID,
    })
    .instruction(),
  "buyer claims SELF + weighted Pioneer after saturation sequence",
);
await send(
  connection,
  sponsor,
  await program.methods
    .claim()
    .accounts({
      wallet: sponsor.publicKey,
      protocol,
      user: sponsorUser,
      vaultAuthority,
      vaultToken: usdtVault.address,
      destination: sponsorUsdt.address,
      tokenProgram: TOKEN_PROGRAM_ID,
    })
    .instruction(),
  "sponsor claims network from Pioneer saturation sequence",
);

sponsorToken = await getAccount(connection, sponsorUsdt.address, "confirmed", TOKEN_PROGRAM_ID);
buyerToken = await getAccount(connection, buyerUsdt.address, "confirmed", TOKEN_PROGRAM_ID);
treasuryToken = await getAccount(connection, treasuryUsdt.address, "confirmed", TOKEN_PROGRAM_ID);
vaultToken = await getAccount(connection, usdtVault.address, "confirmed", TOKEN_PROGRAM_ID);
const sponsorAfter = await program.account.userState.fetch(sponsorUser);
const buyerAfter = await program.account.userState.fetch(buyerUser);

invariant(sponsorToken.amount === 15_920_000_000n, "final sponsor token balance mismatch");
invariant(buyerToken.amount === 53_208_800_000n, "final buyer token balance mismatch");
invariant(treasuryToken.amount === 36_981_200_000n, "final treasury token balance mismatch");
invariant(vaultToken.amount === 0n, "all final Pioneer smoke liabilities must be fully claimable and clear the vault");
invariant(asBigInt(sponsorAfter.lifetimeClaimedUsdt) === 15_920_000_000n, "sponsor lifetime claimed metric mismatch");
invariant(asBigInt(buyerAfter.lifetimeClaimedUsdt) === 53_208_800_000n, "buyer lifetime claimed metric mismatch");
invariant(asBigInt(buyerAfter.pioneerPositions) === 100n, "buyer must retain exactly 100 Pioneer positions after claim");

const total = sponsorToken.amount + buyerToken.amount + treasuryToken.amount + vaultToken.amount;
invariant(total === 106_110n * TOKEN_SCALE, "end-to-end conservation must equal exactly 106110 minted test USDT");

console.log("DEVNET TRANSACTION SMOKE: PASS");
console.log(JSON.stringify({
  programId: programId.toBase58(),
  protocol: protocol.toBase58(),
  sponsor: sponsor.publicKey.toBase58(),
  sponsorUser: sponsorUser.toBase58(),
  buyer: payer.publicKey.toBase58(),
  buyerUser: buyerUser.toBase58(),
  usdtMint: usdtMint.toBase58(),
  usdcMint: usdcMint.toBase58(),
  treasury: treasury.publicKey.toBase58(),
  pioneerPositionsAssigned: asBigInt(protocolState.pioneerPositionsAssigned).toString(),
  buyerPioneerPositions: asBigInt(buyerAfter.pioneerPositions).toString(),
  finalSponsorAtomic: sponsorToken.amount.toString(),
  finalBuyerAtomic: buyerToken.amount.toString(),
  finalTreasuryAtomic: treasuryToken.amount.toString(),
  finalVaultAtomic: vaultToken.amount.toString(),
}, null, 2));