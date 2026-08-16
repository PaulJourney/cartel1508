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

const fundSponsor = SystemProgram.transfer({
  fromPubkey: payer.publicKey,
  toPubkey: sponsor.publicKey,
  lamports: Math.floor(0.05 * LAMPORTS_PER_SOL),
});
await send(connection, payer, fundSponsor, "fund sponsor devnet gas");

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
const initializeIx = await program.methods
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
  .instruction();
await send(connection, payer, initializeIx, "initialize final core");
await waitUntilChainTime(connection, openAt);

const registerSponsorIx = await program.methods
  .register()
  .accounts({
    wallet: sponsor.publicKey,
    protocol,
    referrerWallet: ZERO_PUBKEY,
    referrer: technicalRoot,
    user: sponsorUser,
    systemProgram: SystemProgram.programId,
  })
  .instruction();
await send(connection, sponsor, registerSponsorIx, "register sponsor / Pioneer #1");

const rootEight = Object.fromEntries(
  Array.from({ length: 8 }, (_, i) => [`upline${i + 1}`, technicalRoot]),
);
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
    systemProgram: SystemProgram.programId,
  })
  .instruction();
await send(connection, sponsor, sponsorPurchaseIx, "sponsor buys 10 units / SELF activates");

let sponsorToken = await getAccount(connection, sponsorUsdt.address, "confirmed", TOKEN_PROGRAM_ID);
let buyerToken = await getAccount(connection, buyerUsdt.address, "confirmed", TOKEN_PROGRAM_ID);
let treasuryToken = await getAccount(connection, treasuryUsdt.address, "confirmed", TOKEN_PROGRAM_ID);
let vaultToken = await getAccount(connection, usdtVault.address, "confirmed", TOKEN_PROGRAM_ID);

invariant(sponsorToken.amount === 0n, "sponsor activation must spend exactly 10 USDT");
invariant(buyerToken.amount === 100n * TOKEN_SCALE, "buyer funds must remain untouched before buyer purchase");
invariant(treasuryToken.amount === 4_998_000n, "sponsor activation treasury amount mismatch");
invariant(vaultToken.amount === 5_002_000n, "sponsor SELF + Pioneer liability mismatch");

const registerBuyerIx = await program.methods
  .register()
  .accounts({
    wallet: payer.publicKey,
    protocol,
    referrerWallet: sponsor.publicKey,
    referrer: sponsorUser,
    user: buyerUser,
    systemProgram: SystemProgram.programId,
  })
  .instruction();
await send(connection, payer, registerBuyerIx, "register buyer under sponsor / Pioneer #2");

const buyerPurchaseIx = await program.methods
  .purchaseAndDistribute(new BN(100))
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
    systemProgram: SystemProgram.programId,
  })
  .instruction();
await send(connection, payer, buyerPurchaseIx, "buyer buys 100 units / SELF + nine-upline accounting");

sponsorToken = await getAccount(connection, sponsorUsdt.address, "confirmed", TOKEN_PROGRAM_ID);
buyerToken = await getAccount(connection, buyerUsdt.address, "confirmed", TOKEN_PROGRAM_ID);
treasuryToken = await getAccount(connection, treasuryUsdt.address, "confirmed", TOKEN_PROGRAM_ID);
vaultToken = await getAccount(connection, usdtVault.address, "confirmed", TOKEN_PROGRAM_ID);

invariant(buyerToken.amount === 0n, "buyer purchase must spend exactly 100 USDT");
invariant(treasuryToken.amount === 39_958_000n, "combined treasury balance mismatch after buyer purchase");
invariant(vaultToken.amount === 70_042_000n, "combined vault liabilities mismatch after buyer purchase");

const sponsorState = await program.account.userState.fetch(sponsorUser);
const buyerState = await program.account.userState.fetch(buyerUser);
const protocolStateBeforeClaims = await program.account.protocolState.fetch(protocol);

invariant(asBigInt(sponsorState.pioneerId) === 1n, "sponsor must be Pioneer #1");
invariant(asBigInt(buyerState.pioneerId) === 2n, "buyer must be Pioneer #2");
invariant(asBigInt(sponsorState.selfAccruedUsdt) === 5n * TOKEN_SCALE, "sponsor own SELF reward must be exactly 5 USDT");
invariant(asBigInt(sponsorState.networkClaimableUsdt) === 15n * TOKEN_SCALE, "sponsor U1 reward must be exactly 15 USDT");
invariant(asBigInt(buyerState.selfAccruedUsdt) === 50n * TOKEN_SCALE, "buyer SELF reward must be exactly 50 USDT");
invariant(asBigInt(buyerState.lifetimeServiceUnits) === 100n, "buyer must own 100 logical units");
invariant(asBigInt(sponsorState.activeUntil) >= BigInt(await chainUnixTime(connection)), "sponsor must remain ACTIVE");
invariant(asBigInt(buyerState.activeUntil) >= BigInt(await chainUnixTime(connection)), "100-unit purchase must leave buyer ACTIVE");
invariant(asBigInt(protocolStateBeforeClaims.nextUnitId) === 111n, "next global Unit ID must be 111");
invariant(asBigInt(protocolStateBeforeClaims.lifetimeServiceFeesUsdt) === 5_500_000n, "5% service metric mismatch");
invariant(asBigInt(protocolStateBeforeClaims.lifetimeUnallocatedUsdt) === 32_300_000n, "unallocated upper-network metric mismatch");
invariant(asBigInt(protocolStateBeforeClaims.lifetimePioneerUnassignedUsdt) === 2_158_000n, "Pioneer unassigned metric mismatch");

const sponsorClaimIx = await program.methods
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
  .instruction();
await send(connection, sponsor, sponsorClaimIx, "sponsor pull-claims SELF + U1 + Pioneer");

const buyerClaimIx = await program.methods
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
  .instruction();
await send(connection, payer, buyerClaimIx, "buyer pull-claims SELF + Pioneer");

sponsorToken = await getAccount(connection, sponsorUsdt.address, "confirmed", TOKEN_PROGRAM_ID);
buyerToken = await getAccount(connection, buyerUsdt.address, "confirmed", TOKEN_PROGRAM_ID);
treasuryToken = await getAccount(connection, treasuryUsdt.address, "confirmed", TOKEN_PROGRAM_ID);
vaultToken = await getAccount(connection, usdtVault.address, "confirmed", TOKEN_PROGRAM_ID);
const sponsorAfter = await program.account.userState.fetch(sponsorUser);
const buyerAfter = await program.account.userState.fetch(buyerUser);

invariant(sponsorToken.amount === 20_022_000n, "sponsor claim must pay exactly 20.022 USDT");
invariant(buyerToken.amount === 50_020_000n, "buyer claim must pay exactly 50.020 USDT");
invariant(treasuryToken.amount === 39_958_000n, "claims must not change treasury balance");
invariant(vaultToken.amount === 0n, "all test liabilities must be fully claimed and leave vault empty");
invariant(asBigInt(sponsorAfter.selfAccruedUsdt) === 0n, "sponsor SELF bucket must clear after claim");
invariant(asBigInt(sponsorAfter.networkClaimableUsdt) === 0n, "sponsor network bucket must clear after claim");
invariant(asBigInt(sponsorAfter.lifetimeClaimedUsdt) === 20_022_000n, "sponsor lifetime claim metric mismatch");
invariant(asBigInt(buyerAfter.selfAccruedUsdt) === 0n, "buyer SELF bucket must clear after claim");
invariant(asBigInt(buyerAfter.lifetimeClaimedUsdt) === 50_020_000n, "buyer lifetime claim metric mismatch");

const total = sponsorToken.amount + buyerToken.amount + treasuryToken.amount + vaultToken.amount;
invariant(total === 110n * TOKEN_SCALE, "end-to-end token conservation must equal exactly 110 minted test USDT");

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
  finalSponsorAtomic: sponsorToken.amount.toString(),
  finalBuyerAtomic: buyerToken.amount.toString(),
  finalTreasuryAtomic: treasuryToken.amount.toString(),
  finalVaultAtomic: vaultToken.amount.toString(),
}, null, 2));