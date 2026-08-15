import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { BN, Program } from "@anchor-lang/core";
import {
  Connection,
  Keypair,
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
const WALLET_PATH = (process.env.ANCHOR_WALLET || "~/.config/solana/id.json").replace(/^~(?=$|\/)/, os.homedir());
const ZERO_PUBKEY = new PublicKey("11111111111111111111111111111111");
const TOKEN_SCALE = 1_000_000n;

function invariant(condition, message) {
  if (!condition) throw new Error(`SMOKE INVARIANT FAILED: ${message}`);
}

function readKeypair(filename) {
  const bytes = JSON.parse(fs.readFileSync(filename, "utf8"));
  return Keypair.fromSecretKey(Uint8Array.from(bytes));
}

function u64le(value) {
  const out = Buffer.alloc(8);
  out.writeBigUInt64LE(BigInt(value));
  return out;
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

async function send(connection, payer, instruction, label) {
  const signature = await sendAndConfirmTransaction(
    connection,
    new Transaction().add(instruction),
    [payer],
    { commitment: "confirmed" },
  );
  console.log(`${label}: ${signature}`);
  return signature;
}

const payer = readKeypair(path.resolve(WALLET_PATH));
const idl = JSON.parse(fs.readFileSync(path.resolve(IDL_PATH), "utf8"));
const connection = new Connection(RPC_URL, "confirmed");
const program = new Program(idl, { connection });
const programId = new PublicKey(idl.address);

if (process.env.PROGRAM_ID) {
  invariant(programId.equals(new PublicKey(process.env.PROGRAM_ID)), "IDL Program ID differs from deployed PROGRAM_ID");
}

console.log(`RPC:        ${RPC_URL}`);
console.log(`Program ID: ${programId.toBase58()}`);
console.log(`Payer:      ${payer.publicKey.toBase58()}`);

const treasury = Keypair.generate();
const qualifiedRevenueSource = payer;

const [protocol] = PublicKey.findProgramAddressSync([Buffer.from("protocol")], programId);
const [vaultAuthority] = PublicKey.findProgramAddressSync([Buffer.from("vault-authority")], programId);
const [technicalRoot] = PublicKey.findProgramAddressSync(
  [Buffer.from("user"), ZERO_PUBKEY.toBuffer()],
  programId,
);
const [user] = PublicKey.findProgramAddressSync(
  [Buffer.from("user"), payer.publicKey.toBuffer()],
  programId,
);
const [batch0] = PublicKey.findProgramAddressSync(
  [Buffer.from("batch"), payer.publicKey.toBuffer(), u64le(0n)],
  programId,
);

// Devnet-only mock stablecoins. The program still enforces canonical legacy SPL
// Token accounts and six decimal mints exactly as the production code does.
const usdtMint = await createMint(connection, payer, payer.publicKey, null, 6);
const usdcMint = await createMint(connection, payer, payer.publicKey, null, 6);

const userUsdt = await getOrCreateAssociatedTokenAccount(
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

// Exactly 20 test USDT are minted. Ten pay for 10 service units; the remaining
// ten are used as independently supplied qualified revenue for accounting tests.
await mintTo(connection, payer, usdtMint, userUsdt.address, payer, 20n * TOKEN_SCALE);

const openAt = (await chainUnixTime(connection)) + 4;
const initializeIx = await program.methods
  .initialize(new BN(openAt), qualifiedRevenueSource.publicKey)
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
  .instruction();
await send(connection, payer, initializeIx, "initialize");

await waitUntilChainTime(connection, openAt);

const registerIx = await program.methods
  .register()
  .accounts({
    wallet: payer.publicKey,
    protocol,
    referrerWallet: ZERO_PUBKEY,
    referrer: technicalRoot,
    user,
    systemProgram: SystemProgram.programId,
  })
  .instruction();
await send(connection, payer, registerIx, "register pioneer #1");

const purchaseIx = await program.methods
  .purchaseServiceUnits(new BN(10))
  .accounts({
    wallet: payer.publicKey,
    protocol,
    user,
    userSource: userUsdt.address,
    vaultAuthority,
    usdtVault: usdtVault.address,
    usdcVault: usdcVault.address,
    serviceTreasuryUsdt: treasuryUsdt.address,
    serviceTreasuryUsdc: treasuryUsdc.address,
    batch: batch0,
    tokenProgram: TOKEN_PROGRAM_ID,
    systemProgram: SystemProgram.programId,
  })
  .instruction();
await send(connection, payer, purchaseIx, "purchase 10 service units");

let userToken = await getAccount(connection, userUsdt.address, "confirmed", TOKEN_PROGRAM_ID);
let treasuryToken = await getAccount(connection, treasuryUsdt.address, "confirmed", TOKEN_PROGRAM_ID);
let vaultToken = await getAccount(connection, usdtVault.address, "confirmed", TOKEN_PROGRAM_ID);
invariant(userToken.amount === 10n * TOKEN_SCALE, "purchase must leave exactly 10 test USDT with user");
invariant(treasuryToken.amount === 10n * TOKEN_SCALE, "service-unit purchase must transfer exactly 10 test USDT to treasury");
invariant(vaultToken.amount === 0n, "service-unit purchase must not fund reward vault");

// Beneficiary refers directly to the technical root. Therefore all 43% network
// allocation routes to treasury as unallocated; Pioneer #1 keeps exactly 1/100
// of the 2% Pioneer pool. All ten required upline account slots are supplied,
// but traversal stops after validating the technical root in slot one.
const rootAccounts = Object.fromEntries(
  Array.from({ length: 10 }, (_, i) => [`upline${i + 1}`, technicalRoot]),
);
const revenueIx = await program.methods
  .recordQualifiedRevenue(new BN(10n * TOKEN_SCALE))
  .accounts({
    revenueSource: qualifiedRevenueSource.publicKey,
    protocol,
    sourceToken: userUsdt.address,
    vaultAuthority,
    vaultToken: usdtVault.address,
    serviceTreasuryToken: treasuryUsdt.address,
    beneficiary: user,
    ...rootAccounts,
    tokenProgram: TOKEN_PROGRAM_ID,
  })
  .instruction();
await send(connection, payer, revenueIx, "record 10 qualified-revenue USDT");

userToken = await getAccount(connection, userUsdt.address, "confirmed", TOKEN_PROGRAM_ID);
treasuryToken = await getAccount(connection, treasuryUsdt.address, "confirmed", TOKEN_PROGRAM_ID);
vaultToken = await getAccount(connection, usdtVault.address, "confirmed", TOKEN_PROGRAM_ID);

// On 10 USDT qualified revenue:
// 5.000 direct remains claimable in vault.
// 4.300 network is unallocated because the next ancestor is technical root.
// 0.200 Pioneer pool => 0.002 for Pioneer #1 and 0.198 unassigned to treasury.
// 0.500 service fee goes to treasury.
// Treasury qualified-revenue increment = 4.998; vault liability = 5.002.
invariant(userToken.amount === 0n, "qualified-revenue source must transfer full funded amount into protocol accounting");
invariant(treasuryToken.amount === 14_998_000n, "treasury must hold 10 service + 4.998 qualified allocation");
invariant(vaultToken.amount === 5_002_000n, "vault must retain exactly direct + assigned Pioneer liability");

const preClaimUser = await program.account.userState.fetch(user);
invariant(asBigInt(preClaimUser.pioneerId) === 1n, "first real user must be Pioneer #1");
invariant(asBigInt(preClaimUser.lifetimeServiceUnits) === 10n, "user must own exactly 10 logical service units");
invariant(asBigInt(preClaimUser.directAccruedUsdt) === 5_000_000n, "direct qualified allocation must be 50%");
invariant(asBigInt(preClaimUser.networkClaimableUsdt) === 0n, "root-routed network allocation must not be claimable by beneficiary");
invariant(asBigInt(preClaimUser.activeUntil) >= BigInt(await chainUnixTime(connection)), "10-unit purchase must leave user ACTIVE");

const batch = await program.account.unitBatch.fetch(batch0);
invariant(asBigInt(batch.units) === 10n, "batch must represent 10 logical units");
invariant(asBigInt(batch.firstUnitId) === 1n, "first batch must start at Unit ID 1");
invariant(asBigInt(batch.lastUnitId) === 10n, "first batch must end at Unit ID 10");

const claimIx = await program.methods
  .claim()
  .accounts({
    wallet: payer.publicKey,
    protocol,
    user,
    vaultAuthority,
    vaultToken: usdtVault.address,
    destination: userUsdt.address,
    tokenProgram: TOKEN_PROGRAM_ID,
  })
  .instruction();
await send(connection, payer, claimIx, "claim direct + Pioneer entitlement");

userToken = await getAccount(connection, userUsdt.address, "confirmed", TOKEN_PROGRAM_ID);
treasuryToken = await getAccount(connection, treasuryUsdt.address, "confirmed", TOKEN_PROGRAM_ID);
vaultToken = await getAccount(connection, usdtVault.address, "confirmed", TOKEN_PROGRAM_ID);
const postClaimUser = await program.account.userState.fetch(user);
const protocolState = await program.account.protocolState.fetch(protocol);

invariant(userToken.amount === 5_002_000n, "claim must pay exactly 5.002 USDT to active Pioneer #1");
invariant(treasuryToken.amount === 14_998_000n, "claim must not change treasury balance");
invariant(vaultToken.amount === 0n, "fully claimed funded liabilities must empty test vault");
invariant(asBigInt(postClaimUser.lifetimeClaimedUsdt) === 5_002_000n, "claimed lifetime metric mismatch");
invariant(asBigInt(postClaimUser.directAccruedUsdt) === 0n, "direct bucket must be cleared after claim");
invariant(asBigInt(protocolState.nextUnitId) === 11n, "next global Unit ID must be 11 after ten units");
invariant(asBigInt(protocolState.lifetimeServiceFeesUsdt) === 500_000n, "qualified service fee metric must equal 5%");
invariant(asBigInt(protocolState.lifetimeUnallocatedUsdt) === 4_300_000n, "root-routed network metric must equal 43%");
invariant(asBigInt(protocolState.lifetimePioneerUnassignedUsdt) === 198_000n, "99 unassigned Pioneer shares must route to treasury");

const total = userToken.amount + treasuryToken.amount + vaultToken.amount;
invariant(total === 20n * TOKEN_SCALE, "end-to-end token conservation must equal exactly 20 minted test USDT");

console.log("DEVNET TRANSACTION SMOKE: PASS");
console.log(JSON.stringify({
  programId: programId.toBase58(),
  protocol: protocol.toBase58(),
  user: user.toBase58(),
  batch0: batch0.toBase58(),
  usdtMint: usdtMint.toBase58(),
  usdcMint: usdcMint.toBase58(),
  treasury: treasury.publicKey.toBase58(),
  finalUserAtomic: userToken.amount.toString(),
  finalTreasuryAtomic: treasuryToken.amount.toString(),
  finalVaultAtomic: vaultToken.amount.toString(),
}, null, 2));
