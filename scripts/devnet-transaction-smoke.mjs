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

// Fund the sponsor only with devnet SOL needed to sign its registration,
// activation purchase and later pull claim. No protocol account subsidizes gas.
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
const [sponsorBatch0] = PublicKey.findProgramAddressSync(
  [Buffer.from("batch"), sponsor.publicKey.toBuffer(), u64le(0n)],
  programId,
);
const [buyerBatch0] = PublicKey.findProgramAddressSync(
  [Buffer.from("batch"), payer.publicKey.toBuffer(), u64le(0n)],
  programId,
);

// Devnet-only mock stablecoins. The program still enforces legacy SPL Token,
// canonical ATAs and six-decimal mints exactly as the production core does.
const usdtMint = await createMint(connection, payer, payer.publicKey, null, 6);
const usdcMint = await createMint(connection, payer, payer.publicKey, null, 6);

const sponsorUsdt = await getOrCreateAssociatedTokenAccount(
  connection,
  payer,
  usdtMint,
  sponsor.publicKey,
  false,
  "confirmed",
  undefined,
  TOKEN_PROGRAM_ID,
);
const buyerUsdt = await getOrCreateAssociatedTokenAccount(
  connection,
  payer,
  usdtMint,
  payer.publicKey,
  false,
  "confirmed",
  undefined,
  TOKEN_PROGRAM_ID,
);
const usdtVault = await getOrCreateAssociatedTokenAccount(
  connection,
  payer,
  usdtMint,
  vaultAuthority,
  true,
  "confirmed",
  undefined,
  TOKEN_PROGRAM_ID,
);
const usdcVault = await getOrCreateAssociatedTokenAccount(
  connection,
  payer,
  usdcMint,
  vaultAuthority,
  true,
  "confirmed",
  undefined,
  TOKEN_PROGRAM_ID,
);
const treasuryUsdt = await getOrCreateAssociatedTokenAccount(
  connection,
  payer,
  usdtMint,
  treasury.publicKey,
  false,
  "confirmed",
  undefined,
  TOKEN_PROGRAM_ID,
);
const treasuryUsdc = await getOrCreateAssociatedTokenAccount(
  connection,
  payer,
  usdcMint,
  treasury.publicKey,
  false,
  "confirmed",
  undefined,
  TOKEN_PROGRAM_ID,
);

// Exactly 110 test USDT are minted: sponsor activates with 10; buyer purchases 100.
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

const rootNine = Object.fromEntries(
  Array.from({ length: 9 }, (_, i) => [`upline${i + 1}`, technicalRoot]),
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
    ...rootNine,
    batch: sponsorBatch0,
    tokenProgram: TOKEN_PROGRAM_ID,
    systemProgram: SystemProgram.programId,
  })
  .instruction();
await send(connection, sponsor, sponsorPurchaseIx, "sponsor buys 10 units / becomes ACTIVE");

let sponsorToken = await getAccount(connection, sponsorUsdt.address, "confirmed", TOKEN_PROGRAM_ID);
let buyerToken = await getAccount(connection, buyerUsdt.address, "confirmed", TOKEN_PROGRAM_ID);
let treasuryToken = await getAccount(connection, treasuryUsdt.address, "confirmed", TOKEN_PROGRAM_ID);
let vaultToken = await getAccount(connection, usdtVault.address, "confirmed", TOKEN_PROGRAM_ID);

// Sponsor is under the technical root. Its 50% direct and all 43% depth are therefore
// unallocated; of the 2% Pioneer pool, Pioneer #1 owns 1/100 = 0.002 USDT.
invariant(sponsorToken.amount === 0n, "sponsor activation must spend exactly 10 USDT");
invariant(buyerToken.amount === 100n * TOKEN_SCALE, "buyer funds must remain untouched before buyer purchase");
invariant(treasuryToken.amount === 9_998_000n, "sponsor activation treasury amount mismatch");
invariant(vaultToken.amount === 2_000n, "sponsor activation Pioneer liability mismatch");

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
    upline9: technicalRoot,
    batch: buyerBatch0,
    tokenProgram: TOKEN_PROGRAM_ID,
    systemProgram: SystemProgram.programId,
  })
  .instruction();
await send(connection, payer, buyerPurchaseIx, "buyer buys 100 units + creates all referral accounting");

sponsorToken = await getAccount(connection, sponsorUsdt.address, "confirmed", TOKEN_PROGRAM_ID);
buyerToken = await getAccount(connection, buyerUsdt.address, "confirmed", TOKEN_PROGRAM_ID);
treasuryToken = await getAccount(connection, treasuryUsdt.address, "confirmed", TOKEN_PROGRAM_ID);
vaultToken = await getAccount(connection, usdtVault.address, "confirmed", TOKEN_PROGRAM_ID);

// Buyer purchase 100 USDT:
// - 50.000 direct liability to sponsor L1;
// - 43.000 unallocated network because L2 is technical root;
// - 2.000 Pioneer pool => 0.020 each to two registered Pioneers, 1.960 unassigned;
// - 5.000 service.
// Buyer-purchase treasury increment = 49.960; new claim liability = 50.040.
invariant(buyerToken.amount === 0n, "buyer purchase must spend exactly 100 USDT");
invariant(treasuryToken.amount === 59_958_000n, "combined treasury balance mismatch after buyer purchase");
invariant(vaultToken.amount === 50_042_000n, "combined vault liabilities mismatch after buyer purchase");

const sponsorState = await program.account.userState.fetch(sponsorUser);
const buyerState = await program.account.userState.fetch(buyerUser);
const sponsorBatch = await program.account.unitBatch.fetch(sponsorBatch0);
const buyerBatch = await program.account.unitBatch.fetch(buyerBatch0);
const protocolStateBeforeClaims = await program.account.protocolState.fetch(protocol);

invariant(asBigInt(sponsorState.pioneerId) === 1n, "sponsor must be Pioneer #1");
invariant(asBigInt(buyerState.pioneerId) === 2n, "buyer must be Pioneer #2");
invariant(asBigInt(sponsorState.directAccruedUsdt) === 50n * TOKEN_SCALE, "sponsor direct reward must be exactly 50%");
invariant(asBigInt(sponsorState.networkClaimableUsdt) === 0n, "L1 sponsor must not receive a duplicate network share");
invariant(asBigInt(buyerState.lifetimeServiceUnits) === 100n, "buyer must own 100 logical units");
invariant(asBigInt(sponsorState.activeUntil) >= BigInt(await chainUnixTime(connection)), "sponsor must remain ACTIVE");
invariant(asBigInt(buyerState.activeUntil) >= BigInt(await chainUnixTime(connection)), "100-unit purchase must leave buyer ACTIVE");
invariant(asBigInt(sponsorBatch.firstUnitId) === 1n && asBigInt(sponsorBatch.lastUnitId) === 10n, "sponsor unit IDs must be 1..10");
invariant(asBigInt(buyerBatch.firstUnitId) === 11n && asBigInt(buyerBatch.lastUnitId) === 110n, "buyer unit IDs must be 11..110");
invariant(asBigInt(protocolStateBeforeClaims.nextUnitId) === 111n, "next global Unit ID must be 111");
invariant(asBigInt(protocolStateBeforeClaims.lifetimeServiceFeesUsdt) === 5_500_000n, "5% service metric mismatch");
invariant(asBigInt(protocolStateBeforeClaims.lifetimeUnallocatedUsdt) === 102_300_000n, "unallocated direct/network metric mismatch");
invariant(asBigInt(protocolStateBeforeClaims.lifetimePioneerUnassignedUsdt) === 2_158_000n, "Pioneer unassigned metric mismatch");

// Sponsor independently signs and pays gas for one pull claim containing all of its
// accumulated direct + Pioneer entitlement: 50 + 0.002 + 0.020 = 50.022 USDT.
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
await send(connection, sponsor, sponsorClaimIx, "sponsor pull-claims 50% direct + Pioneer");

// Buyer independently claims its 0.020 USDT Pioneer entitlement.
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
await send(connection, payer, buyerClaimIx, "buyer pull-claims Pioneer entitlement");

sponsorToken = await getAccount(connection, sponsorUsdt.address, "confirmed", TOKEN_PROGRAM_ID);
buyerToken = await getAccount(connection, buyerUsdt.address, "confirmed", TOKEN_PROGRAM_ID);
treasuryToken = await getAccount(connection, treasuryUsdt.address, "confirmed", TOKEN_PROGRAM_ID);
vaultToken = await getAccount(connection, usdtVault.address, "confirmed", TOKEN_PROGRAM_ID);
const sponsorAfter = await program.account.userState.fetch(sponsorUser);
const buyerAfter = await program.account.userState.fetch(buyerUser);

invariant(sponsorToken.amount === 50_022_000n, "sponsor claim must pay exactly 50.022 USDT");
invariant(buyerToken.amount === 20_000n, "buyer Pioneer claim must pay exactly 0.020 USDT");
invariant(treasuryToken.amount === 59_958_000n, "claims must not change treasury balance");
invariant(vaultToken.amount === 0n, "all test liabilities must be fully claimable and leave vault empty");
invariant(asBigInt(sponsorAfter.directAccruedUsdt) === 0n, "sponsor direct bucket must clear after claim");
invariant(asBigInt(sponsorAfter.lifetimeClaimedUsdt) === 50_022_000n, "sponsor lifetime claim metric mismatch");
invariant(asBigInt(buyerAfter.lifetimeClaimedUsdt) === 20_000n, "buyer lifetime claim metric mismatch");

const total = sponsorToken.amount + buyerToken.amount + treasuryToken.amount + vaultToken.amount;
invariant(total === 110n * TOKEN_SCALE, "end-to-end token conservation must equal exactly 110 minted test USDT");

console.log("DEVNET TRANSACTION SMOKE: PASS");
console.log(
  JSON.stringify(
    {
      programId: programId.toBase58(),
      protocol: protocol.toBase58(),
      sponsor: sponsor.publicKey.toBase58(),
      sponsorUser: sponsorUser.toBase58(),
      buyer: payer.publicKey.toBase58(),
      buyerUser: buyerUser.toBase58(),
      sponsorBatch0: sponsorBatch0.toBase58(),
      buyerBatch0: buyerBatch0.toBase58(),
      usdtMint: usdtMint.toBase58(),
      usdcMint: usdcMint.toBase58(),
      treasury: treasury.publicKey.toBase58(),
      finalSponsorAtomic: sponsorToken.amount.toString(),
      finalBuyerAtomic: buyerToken.amount.toString(),
      finalTreasuryAtomic: treasuryToken.amount.toString(),
      finalVaultAtomic: vaultToken.amount.toString(),
    },
    null,
    2,
  ),
);
