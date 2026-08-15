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

const RPC_URL = process.env.ANCHOR_PROVIDER_URL || "http://127.0.0.1:8899";
const WALLET_PATH = (process.env.ANCHOR_WALLET || "~/.config/solana/id.json").replace(
  /^~(?=$|\/)/,
  os.homedir(),
);
const REFERRAL_IDL = process.env.REFERRAL_IDL || "target/idl/service_referral_protocol.json";
const ADAPTER_IDL = process.env.ADAPTER_IDL || "target/idl/revenue_adapter.json";
const QUALIFICATION_IDL = process.env.QUALIFICATION_IDL || "target/idl/revenue_qualification.json";
const ZERO_PUBKEY = new PublicKey("11111111111111111111111111111111");
const TOKEN_SCALE = 1_000_000n;
const EVENT_DOMAIN = Buffer.from("qualified-revenue-v1");
const RECEIPT_SEED = Buffer.from("revenue-receipt");
const REVENUE_AUTHORITY_SEED = Buffer.from("revenue-authority");
const CONFIG_SEED = Buffer.from("adapter-config");
const QUALIFIER_AUTHORITY_SEED = Buffer.from("qualified-revenue-authority");

function invariant(condition, message) {
  if (!condition) throw new Error(`QUALIFICATION SMOKE INVARIANT FAILED: ${message}`);
}

function readKeypair(filename) {
  return Keypair.fromSecretKey(
    Uint8Array.from(JSON.parse(fs.readFileSync(path.resolve(filename), "utf8"))),
  );
}

function readIdl(filename) {
  return JSON.parse(fs.readFileSync(path.resolve(filename), "utf8"));
}

function u64le(value) {
  const out = Buffer.alloc(8);
  out.writeBigUInt64LE(BigInt(value));
  return out;
}

function asBigInt(value) {
  return BigInt(value.toString());
}

function eventId(qualificationProgramId, payer, beneficiary, mint, amount, evidenceHash) {
  return PublicKey.findProgramAddressSync(
    [EVENT_DOMAIN, evidenceHash, payer.toBuffer(), beneficiary.toBuffer(), mint.toBuffer(), u64le(amount)],
    qualificationProgramId,
  )[0].toBuffer();
}

async function chainUnixTime(connection) {
  const slot = await connection.getSlot("confirmed");
  const blockTime = await connection.getBlockTime(slot);
  return blockTime ?? Math.floor(Date.now() / 1000);
}

async function waitUntil(connection, unixTime) {
  while ((await chainUnixTime(connection)) < unixTime) {
    await new Promise((resolve) => setTimeout(resolve, 250));
  }
}

async function sendIx(connection, feePayer, instruction, signers, label) {
  const tx = new Transaction().add(instruction);
  const signature = await sendAndConfirmTransaction(connection, tx, [feePayer, ...signers], {
    commitment: "confirmed",
  });
  console.log(`${label}: ${signature}`);
  return signature;
}

async function expectFailure(connection, feePayer, instruction, signers, label) {
  try {
    await sendAndConfirmTransaction(
      connection,
      new Transaction().add(instruction),
      [feePayer, ...signers],
      { commitment: "confirmed" },
    );
  } catch (error) {
    console.log(`${label}: rejected as expected`);
    return error;
  }
  throw new Error(`${label}: transaction unexpectedly succeeded`);
}

async function tokenAmount(connection, address) {
  return (await getAccount(connection, address, "confirmed", TOKEN_PROGRAM_ID)).amount;
}

const connection = new Connection(RPC_URL, "confirmed");
const payer = readKeypair(WALLET_PATH);
const referralIdl = readIdl(REFERRAL_IDL);
const adapterIdl = readIdl(ADAPTER_IDL);
const qualificationIdl = readIdl(QUALIFICATION_IDL);
const referral = new Program(referralIdl, { connection });
const adapter = new Program(adapterIdl, { connection });
const qualification = new Program(qualificationIdl, { connection });
const referralId = new PublicKey(referralIdl.address);
const adapterId = new PublicKey(adapterIdl.address);
const qualificationId = new PublicKey(qualificationIdl.address);

if (process.env.REFERRAL_PROGRAM_ID) {
  invariant(referralId.equals(new PublicKey(process.env.REFERRAL_PROGRAM_ID)), "referral IDL address mismatch");
}
if (process.env.ADAPTER_PROGRAM_ID) {
  invariant(adapterId.equals(new PublicKey(process.env.ADAPTER_PROGRAM_ID)), "adapter IDL address mismatch");
}
if (process.env.QUALIFICATION_PROGRAM_ID) {
  invariant(
    qualificationId.equals(new PublicKey(process.env.QUALIFICATION_PROGRAM_ID)),
    "qualification IDL address mismatch",
  );
}

console.log(`RPC:           ${RPC_URL}`);
console.log(`Referral:      ${referralId.toBase58()}`);
console.log(`Adapter:       ${adapterId.toBase58()}`);
console.log(`Qualification: ${qualificationId.toBase58()}`);

const treasury = Keypair.generate();
const userWallet = Keypair.generate();
const customer = Keypair.generate();

// Fund the two transaction-signing actors from the local validator payer.
for (const recipient of [userWallet.publicKey, customer.publicKey]) {
  await sendIx(
    connection,
    payer,
    SystemProgram.transfer({ fromPubkey: payer.publicKey, toPubkey: recipient, lamports: 2_000_000_000 }),
    [],
    `fund ${recipient.toBase58().slice(0, 8)}`,
  );
}

// Local-only six-decimal mock mints. Production builds remain pinned/fail-closed separately.
const usdtMint = await createMint(connection, payer, payer.publicKey, null, 6);
const usdcMint = await createMint(connection, payer, payer.publicKey, null, 6);

const [protocol] = PublicKey.findProgramAddressSync([Buffer.from("protocol")], referralId);
const [vaultAuthority] = PublicKey.findProgramAddressSync([Buffer.from("vault-authority")], referralId);
const [technicalRoot] = PublicKey.findProgramAddressSync(
  [Buffer.from("user"), ZERO_PUBKEY.toBuffer()],
  referralId,
);
const [user] = PublicKey.findProgramAddressSync(
  [Buffer.from("user"), userWallet.publicKey.toBuffer()],
  referralId,
);
const [batch0] = PublicKey.findProgramAddressSync(
  [Buffer.from("batch"), userWallet.publicKey.toBuffer(), u64le(0n)],
  referralId,
);
const [adapterConfig] = PublicKey.findProgramAddressSync([CONFIG_SEED], adapterId);
const [revenueAuthority] = PublicKey.findProgramAddressSync([REVENUE_AUTHORITY_SEED], adapterId);
const [qualificationAuthority] = PublicKey.findProgramAddressSync(
  [QUALIFIER_AUTHORITY_SEED],
  qualificationId,
);

const vaultUsdt = await getOrCreateAssociatedTokenAccount(
  connection,
  payer,
  usdtMint,
  vaultAuthority,
  true,
  "confirmed",
  undefined,
  TOKEN_PROGRAM_ID,
);
const vaultUsdc = await getOrCreateAssociatedTokenAccount(
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
const userUsdc = await getOrCreateAssociatedTokenAccount(
  connection,
  payer,
  usdcMint,
  userWallet.publicKey,
  false,
  "confirmed",
  undefined,
  TOKEN_PROGRAM_ID,
);
const customerUsdc = await getOrCreateAssociatedTokenAccount(
  connection,
  payer,
  usdcMint,
  customer.publicKey,
  false,
  "confirmed",
  undefined,
  TOKEN_PROGRAM_ID,
);
const revenueUsdc = await getOrCreateAssociatedTokenAccount(
  connection,
  payer,
  usdcMint,
  revenueAuthority,
  true,
  "confirmed",
  undefined,
  TOKEN_PROGRAM_ID,
);

await mintTo(connection, payer, usdcMint, userUsdc.address, payer, 10n * TOKEN_SCALE);
await mintTo(connection, payer, usdcMint, customerUsdc.address, payer, 300n * TOKEN_SCALE);

const openAt = (await chainUnixTime(connection)) + 1;
const initReferral = await referral.methods
  .initialize(new BN(openAt), revenueAuthority)
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
await sendIx(connection, payer, initReferral, [], "initialize referral");

const initAdapter = await adapter.methods
  .initialize(referralId, qualificationId, usdtMint, usdcMint)
  .accounts({
    initializer: payer.publicKey,
    config: adapterConfig,
    revenueAuthority,
    qualificationAuthority,
    systemProgram: SystemProgram.programId,
  })
  .instruction();
await sendIx(connection, payer, initAdapter, [], "initialize adapter");

await waitUntil(connection, openAt);

const registerIx = await referral.methods
  .register()
  .accounts({
    wallet: userWallet.publicKey,
    protocol,
    referrerWallet: ZERO_PUBKEY,
    referrer: technicalRoot,
    user,
    systemProgram: SystemProgram.programId,
  })
  .instruction();
await sendIx(connection, userWallet, registerIx, [], "register pioneer #1");

const purchaseIx = await referral.methods
  .purchaseServiceUnits(new BN(10))
  .accounts({
    wallet: userWallet.publicKey,
    protocol,
    user,
    userSource: userUsdc.address,
    vaultAuthority,
    usdtVault: vaultUsdt.address,
    usdcVault: vaultUsdc.address,
    serviceTreasuryUsdt: treasuryUsdt.address,
    serviceTreasuryUsdc: treasuryUsdc.address,
    batch: batch0,
    tokenProgram: TOKEN_PROGRAM_ID,
    systemProgram: SystemProgram.programId,
  })
  .instruction();
await sendIx(connection, userWallet, purchaseIx, [], "activate with 10 units");

invariant((await tokenAmount(connection, userUsdc.address)) === 0n, "activation funds must be consumed");
invariant((await tokenAmount(connection, treasuryUsdc.address)) === 10n * TOKEN_SCALE, "10 service units must reach treasury");
invariant((await tokenAmount(connection, vaultUsdc.address)) === 0n, "service purchase must not fund reward vault");

const revenueAmount = 100n * TOKEN_SCALE;
const direct = 50n * TOKEN_SCALE;
const pioneerAssigned = (2n * TOKEN_SCALE) / 100n;
const treasuryQualifiedDelta = 43n * TOKEN_SCALE + 5n * TOKEN_SCALE + (2n * TOKEN_SCALE - pioneerAssigned);
const userLiability = direct + pioneerAssigned;
const rootAccounts = Object.fromEntries(Array.from({ length: 10 }, (_, i) => [`upline${i + 1}`, technicalRoot]));

const evidenceHash1 = Buffer.alloc(32, 1);
const event1 = eventId(qualificationId, customer.publicKey, user, usdcMint, revenueAmount, evidenceHash1);
const [receipt1] = PublicKey.findProgramAddressSync([RECEIPT_SEED, event1], adapterId);
const qualify1 = await qualification.methods
  .qualifyPaymentAndRoute([...evidenceHash1], new BN(revenueAmount.toString()))
  .accounts({
    payer: customer.publicKey,
    payerSourceToken: customerUsdc.address,
    qualificationAuthority,
    adapterConfig,
    revenueAuthority,
    revenueSourceToken: revenueUsdc.address,
    adapterReceipt: receipt1,
    adapterProgram: adapterId,
    referralProgram: referralId,
    protocol,
    vaultAuthority,
    vaultToken: vaultUsdc.address,
    serviceTreasuryToken: treasuryUsdc.address,
    beneficiary: user,
    ...rootAccounts,
    tokenProgram: TOKEN_PROGRAM_ID,
    systemProgram: SystemProgram.programId,
  })
  .instruction();
await sendIx(connection, customer, qualify1, [], "qualify 100 USDC real payment");

invariant((await tokenAmount(connection, customerUsdc.address)) === 200n * TOKEN_SCALE, "customer must spend exactly 100 USDC");
invariant((await tokenAmount(connection, revenueUsdc.address)) === 0n, "adapter revenue ATA must be drained by downstream accounting");
invariant((await tokenAmount(connection, vaultUsdc.address)) === userLiability, "vault must retain only user liabilities");
invariant(
  (await tokenAmount(connection, treasuryUsdc.address)) === 10n * TOKEN_SCALE + treasuryQualifiedDelta,
  "treasury qualified allocation mismatch",
);
invariant((await connection.getAccountInfo(receipt1, "confirmed")) !== null, "successful event receipt must exist");

const userState = await referral.account.userState.fetch(user);
invariant(asBigInt(userState.pioneerId) === 1n, "first registered user must be Pioneer #1");
invariant(asBigInt(userState.directAccruedUsdc) === direct, "direct reward must equal 50%");

// Exact replay: adapter receipt already exists. The qualification program transfers first,
// so this proves the entire outer transaction rolls the transfer back on the failed CPI.
const replayCustomerBefore = await tokenAmount(connection, customerUsdc.address);
const replayTreasuryBefore = await tokenAmount(connection, treasuryUsdc.address);
const replayVaultBefore = await tokenAmount(connection, vaultUsdc.address);
await expectFailure(connection, customer, qualify1, [], "exact replay");
invariant((await tokenAmount(connection, customerUsdc.address)) === replayCustomerBefore, "replay must roll back customer transfer");
invariant((await tokenAmount(connection, treasuryUsdc.address)) === replayTreasuryBefore, "replay must not change treasury");
invariant((await tokenAmount(connection, vaultUsdc.address)) === replayVaultBefore, "replay must not change vault");

// Referral executable substitution: the new qualification preflight must reject a
// different executable before value enters the adapter path. The account below is a
// real executable (the qualification program itself), so this specifically exercises
// the immutable referral binding rather than merely failing an executable check.
const evidenceHash3 = Buffer.alloc(32, 3);
const event3 = eventId(qualificationId, customer.publicKey, user, usdcMint, revenueAmount, evidenceHash3);
const [receipt3] = PublicKey.findProgramAddressSync([RECEIPT_SEED, event3], adapterId);
const substitutedReferral = await qualification.methods
  .qualifyPaymentAndRoute([...evidenceHash3], new BN(revenueAmount.toString()))
  .accounts({
    payer: customer.publicKey,
    payerSourceToken: customerUsdc.address,
    qualificationAuthority,
    adapterConfig,
    revenueAuthority,
    revenueSourceToken: revenueUsdc.address,
    adapterReceipt: receipt3,
    adapterProgram: adapterId,
    referralProgram: qualificationId,
    protocol,
    vaultAuthority,
    vaultToken: vaultUsdc.address,
    serviceTreasuryToken: treasuryUsdc.address,
    beneficiary: user,
    ...rootAccounts,
    tokenProgram: TOKEN_PROGRAM_ID,
    systemProgram: SystemProgram.programId,
  })
  .instruction();
const substitutionCustomerBefore = await tokenAmount(connection, customerUsdc.address);
const substitutionTreasuryBefore = await tokenAmount(connection, treasuryUsdc.address);
const substitutionVaultBefore = await tokenAmount(connection, vaultUsdc.address);
const substitutionRevenueBefore = await tokenAmount(connection, revenueUsdc.address);
await expectFailure(connection, customer, substitutedReferral, [], "referral executable substitution");
invariant((await tokenAmount(connection, customerUsdc.address)) === substitutionCustomerBefore, "referral substitution must not charge customer");
invariant((await tokenAmount(connection, treasuryUsdc.address)) === substitutionTreasuryBefore, "referral substitution must not alter treasury");
invariant((await tokenAmount(connection, vaultUsdc.address)) === substitutionVaultBefore, "referral substitution must not alter vault");
invariant((await tokenAmount(connection, revenueUsdc.address)) === substitutionRevenueBefore, "referral substitution must not alter Revenue Authority ATA");
invariant((await connection.getAccountInfo(receipt3, "confirmed")) === null, "referral substitution must not create a receipt");

// Fresh receipt + deliberately wrong ancestry. Adapter creates the receipt before referral CPI;
// downstream failure must therefore roll back both the customer's token transfer and receipt creation.
const evidenceHash2 = Buffer.alloc(32, 2);
const event2 = eventId(qualificationId, customer.publicKey, user, usdcMint, revenueAmount, evidenceHash2);
const [receipt2] = PublicKey.findProgramAddressSync([RECEIPT_SEED, event2], adapterId);
const wrongRootAccounts = { ...rootAccounts, upline1: user };
const badDownstream = await qualification.methods
  .qualifyPaymentAndRoute([...evidenceHash2], new BN(revenueAmount.toString()))
  .accounts({
    payer: customer.publicKey,
    payerSourceToken: customerUsdc.address,
    qualificationAuthority,
    adapterConfig,
    revenueAuthority,
    revenueSourceToken: revenueUsdc.address,
    adapterReceipt: receipt2,
    adapterProgram: adapterId,
    referralProgram: referralId,
    protocol,
    vaultAuthority,
    vaultToken: vaultUsdc.address,
    serviceTreasuryToken: treasuryUsdc.address,
    beneficiary: user,
    ...wrongRootAccounts,
    tokenProgram: TOKEN_PROGRAM_ID,
    systemProgram: SystemProgram.programId,
  })
  .instruction();
const failureCustomerBefore = await tokenAmount(connection, customerUsdc.address);
const failureTreasuryBefore = await tokenAmount(connection, treasuryUsdc.address);
const failureVaultBefore = await tokenAmount(connection, vaultUsdc.address);
await expectFailure(connection, customer, badDownstream, [], "deliberate ancestry failure");
invariant((await tokenAmount(connection, customerUsdc.address)) === failureCustomerBefore, "downstream failure must refund by rollback");
invariant((await tokenAmount(connection, treasuryUsdc.address)) === failureTreasuryBefore, "downstream failure must not alter treasury");
invariant((await tokenAmount(connection, vaultUsdc.address)) === failureVaultBefore, "downstream failure must not alter vault");
invariant((await connection.getAccountInfo(receipt2, "confirmed")) === null, "failed event receipt must not survive rollback");

const claimIx = await referral.methods
  .claim()
  .accounts({
    wallet: userWallet.publicKey,
    protocol,
    user,
    vaultAuthority,
    vaultToken: vaultUsdc.address,
    destination: userUsdc.address,
    tokenProgram: TOKEN_PROGRAM_ID,
  })
  .instruction();
await sendIx(connection, userWallet, claimIx, [], "claim direct + pioneer");

const finalUser = await tokenAmount(connection, userUsdc.address);
const finalCustomer = await tokenAmount(connection, customerUsdc.address);
const finalTreasury = await tokenAmount(connection, treasuryUsdc.address);
const finalVault = await tokenAmount(connection, vaultUsdc.address);
const finalRevenue = await tokenAmount(connection, revenueUsdc.address);
const total = finalUser + finalCustomer + finalTreasury + finalVault + finalRevenue;

invariant(finalUser === userLiability, "claim payout mismatch");
invariant(finalCustomer === 200n * TOKEN_SCALE, "only the successful qualification may charge customer");
invariant(finalTreasury === 10n * TOKEN_SCALE + treasuryQualifiedDelta, "final treasury mismatch");
invariant(finalVault === 0n, "claim must empty funded test liability vault");
invariant(finalRevenue === 0n, "revenue ATA must finish empty");
invariant(total === 310n * TOKEN_SCALE, "all minted USDC must be conserved exactly");

console.log("QUALIFICATION LOCALNET SMOKE: PASS");
console.log(
  JSON.stringify(
    {
      referralProgramId: referralId.toBase58(),
      adapterProgramId: adapterId.toBase58(),
      qualificationProgramId: qualificationId.toBase58(),
      protocol: protocol.toBase58(),
      adapterConfig: adapterConfig.toBase58(),
      revenueAuthority: revenueAuthority.toBase58(),
      qualificationAuthority: qualificationAuthority.toBase58(),
      receipt1: receipt1.toBase58(),
      receipt2RolledBack: receipt2.toBase58(),
      receipt3ReferralSubstitutionRejected: receipt3.toBase58(),
      usdcMint: usdcMint.toBase58(),
      finalUserAtomic: finalUser.toString(),
      finalCustomerAtomic: finalCustomer.toString(),
      finalTreasuryAtomic: finalTreasury.toString(),
      finalVaultAtomic: finalVault.toString(),
      finalRevenueAtomic: finalRevenue.toString(),
    },
    null,
    2,
  ),
);
