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
  if (!condition) throw new Error(`SECURITY INVARIANT FAILED: ${message}`);
}

function readKeypair(filename) {
  const bytes = JSON.parse(fs.readFileSync(filename, "utf8"));
  return Keypair.fromSecretKey(Uint8Array.from(bytes));
}

function asBigInt(value) {
  return BigInt(value.toString());
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
  throw new Error(`SECURITY INVARIANT FAILED: expected rejection: ${label}`);
}

const payer = readKeypair(path.resolve(WALLET_PATH));
const idl = JSON.parse(fs.readFileSync(path.resolve(IDL_PATH), "utf8"));
const connection = new Connection(RPC_URL, "confirmed");
const program = new Program(idl, { connection });
const programId = new PublicKey(idl.address);

if (process.env.PROGRAM_ID) {
  invariant(programId.equals(new PublicKey(process.env.PROGRAM_ID)), "IDL Program ID differs from PROGRAM_ID");
}

const [protocol] = PublicKey.findProgramAddressSync([Buffer.from("protocol")], programId);
const [vaultAuthority] = PublicKey.findProgramAddressSync([Buffer.from("vault-authority")], programId);
const [technicalRoot] = PublicKey.findProgramAddressSync(
  [Buffer.from("user"), ZERO_PUBKEY.toBuffer()],
  programId,
);

async function protocolState() {
  return program.account.protocolState.fetch(protocol);
}

async function userState(pda) {
  return program.account.userState.fetch(pda);
}

async function tokenAmount(address) {
  return (await getAccount(connection, address, "confirmed", TOKEN_PROGRAM_ID)).amount;
}

async function ata(owner, mint, allowOwnerOffCurve = false) {
  return (
    await getOrCreateAssociatedTokenAccount(
      connection,
      payer,
      mint,
      owner,
      allowOwnerOffCurve,
      "confirmed",
      undefined,
      TOKEN_PROGRAM_ID,
    )
  ).address;
}

async function fundGas(keypair, sol = 0.025) {
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

function userPda(wallet) {
  return PublicKey.findProgramAddressSync(
    [Buffer.from("user"), wallet.toBuffer()],
    programId,
  )[0];
}

async function registerInstruction(keypair, referrerWallet, referrer, pda = userPda(keypair.publicKey)) {
  return program.methods
    .register()
    .accounts({
      wallet: keypair.publicKey,
      protocol,
      referrerWallet,
      referrer,
      user: pda,
      systemProgram: SystemProgram.programId,
    })
    .instruction();
}

console.log("\n=== SECURITY PHASE A: registration integrity ===");
const beforeRegistration = await protocolState();
const initialRealUsers = asBigInt(beforeRegistration.realUserCount);

const parent = Keypair.generate();
await fundGas(parent);
const parentPda = userPda(parent.publicKey);
await send(
  connection,
  parent,
  await registerInstruction(parent, ZERO_PUBKEY, technicalRoot, parentPda),
  "register security parent",
);
let state = await protocolState();
invariant(asBigInt(state.realUserCount) === initialRealUsers + 1n, "valid registration did not increment count once");

await expectFailure(
  connection,
  parent,
  await registerInstruction(parent, ZERO_PUBKEY, technicalRoot, parentPda),
  "re-registration of same wallet",
);
state = await protocolState();
invariant(asBigInt(state.realUserCount) === initialRealUsers + 1n, "failed re-registration changed realUserCount");

const candidate = Keypair.generate();
await fundGas(candidate);
const candidatePda = userPda(candidate.publicKey);
const unrelatedWallet = Keypair.generate().publicKey;
await expectFailure(
  connection,
  candidate,
  await registerInstruction(candidate, unrelatedWallet, parentPda, candidatePda),
  "spoofed referrer wallet/PDA pair",
);
invariant((await connection.getAccountInfo(candidatePda, "confirmed")) === null, "spoofed registration created UserState");
state = await protocolState();
invariant(asBigInt(state.realUserCount) === initialRealUsers + 1n, "spoofed registration changed realUserCount");

await expectFailure(
  connection,
  candidate,
  await registerInstruction(candidate, candidate.publicKey, candidatePda, candidatePda),
  "structural self-referral",
);
invariant((await connection.getAccountInfo(candidatePda, "confirmed")) === null, "self-referral created UserState");
state = await protocolState();
invariant(asBigInt(state.realUserCount) === initialRealUsers + 1n, "self-referral changed realUserCount");

console.log("\n=== SECURITY PHASE B: purchase treasury-account substitution ===");
const buyer = Keypair.generate();
await fundGas(buyer);
const buyerPda = userPda(buyer.publicKey);
await send(
  connection,
  buyer,
  await registerInstruction(buyer, parent.publicKey, parentPda, buyerPda),
  "register security buyer",
);
state = await protocolState();
invariant(asBigInt(state.realUserCount) === initialRealUsers + 2n, "security buyer registration count mismatch");

const p = await protocolState();
const usdtMint = new PublicKey(p.usdtMint);
const usdcMint = new PublicKey(p.usdcMint);
const treasury = new PublicKey(p.serviceTreasury);
const usdtVault = await ata(vaultAuthority, usdtMint, true);
const usdcVault = await ata(vaultAuthority, usdcMint, true);
const treasuryUsdt = await ata(treasury, usdtMint, false);
const treasuryUsdc = await ata(treasury, usdcMint, false);
const buyerUsdc = await ata(buyer.publicKey, usdcMint, false);
const parentUsdc = await ata(parent.publicKey, usdcMint, false);
const fakeTreasury = Keypair.generate().publicKey;
const fakeTreasuryUsdc = await ata(fakeTreasury, usdcMint, false);

await mintTo(connection, payer, usdcMint, buyerUsdc, payer, 10n * TOKEN_SCALE);

function purchaseAccounts(treasuryUsdcOverride = treasuryUsdc) {
  return {
    wallet: buyer.publicKey,
    protocol,
    user: buyerPda,
    userSource: buyerUsdc,
    vaultAuthority,
    usdtVault,
    usdcVault,
    serviceTreasuryUsdt: treasuryUsdt,
    serviceTreasuryUsdc: treasuryUsdcOverride,
    directReferrer: parentPda,
    upline1: technicalRoot,
    upline2: technicalRoot,
    upline3: technicalRoot,
    upline4: technicalRoot,
    upline5: technicalRoot,
    upline6: technicalRoot,
    upline7: technicalRoot,
    upline8: technicalRoot,
    tokenProgram: TOKEN_PROGRAM_ID,
  };
}

const protocolBeforeSpoof = await protocolState();
const buyerBeforeSpoof = await userState(buyerPda);
const sourceBeforeSpoof = await tokenAmount(buyerUsdc);
const vaultBeforeSpoof = await tokenAmount(usdcVault);
const treasuryBeforeSpoof = await tokenAmount(treasuryUsdc);

await expectFailure(
  connection,
  buyer,
  await program.methods
    .purchaseAndDistribute(new BN(10))
    .accounts(purchaseAccounts(fakeTreasuryUsdc))
    .instruction(),
  "non-canonical service Treasury ATA",
);

const protocolAfterSpoof = await protocolState();
const buyerAfterSpoof = await userState(buyerPda);
invariant(asBigInt(protocolAfterSpoof.nextUnitId) === asBigInt(protocolBeforeSpoof.nextUnitId), "Treasury spoof advanced Unit ID");
invariant(asBigInt(buyerAfterSpoof.nextPurchaseIndex) === asBigInt(buyerBeforeSpoof.nextPurchaseIndex), "Treasury spoof advanced purchase index");
invariant(await tokenAmount(buyerUsdc) === sourceBeforeSpoof, "Treasury spoof moved buyer tokens");
invariant(await tokenAmount(usdcVault) === vaultBeforeSpoof, "Treasury spoof changed vault");
invariant(await tokenAmount(treasuryUsdc) === treasuryBeforeSpoof, "Treasury spoof changed canonical Treasury");
invariant(await tokenAmount(fakeTreasuryUsdc) === 0n, "Treasury spoof funded fake Treasury");

await send(
  connection,
  buyer,
  await program.methods
    .purchaseAndDistribute(new BN(10))
    .accounts(purchaseAccounts())
    .instruction(),
  "valid security 10-unit purchase",
);

const buyerAfterPurchase = await userState(buyerPda);
const protocolAfterPurchase = await protocolState();
invariant(asBigInt(buyerAfterPurchase.activeWeeksStarted) === 1n, "buyer did not become ACTIVE");
invariant(asBigInt(buyerAfterPurchase.nextPurchaseIndex) === 1n, "valid purchase index mismatch");
invariant(asBigInt(protocolAfterPurchase.nextUnitId) === asBigInt(protocolBeforeSpoof.nextUnitId) + 10n, "valid purchase Unit ID delta mismatch");
invariant(await tokenAmount(buyerUsdc) === 0n, "valid purchase did not consume exact 10 USDC");
invariant(await tokenAmount(treasuryUsdc) - treasuryBeforeSpoof === 4_800_000n, "valid purchase Treasury delta must be 4.8 USDC with saturated Pioneer pool");
invariant(await tokenAmount(usdcVault) - vaultBeforeSpoof === 5_200_000n, "valid purchase vault liability must be 5.2 USDC before buyer claim");

console.log("\n=== SECURITY PHASE C: claim signer/destination/replay integrity ===");
const lifetimeBeforeNegatives = asBigInt(buyerAfterPurchase.lifetimeClaimedUsdc);
const buyerBalanceBeforeNegatives = await tokenAmount(buyerUsdc);
const parentBalanceBeforeNegatives = await tokenAmount(parentUsdc);
const vaultBeforeNegatives = await tokenAmount(usdcVault);

await expectFailure(
  connection,
  parent,
  await program.methods
    .claim()
    .accounts({
      wallet: parent.publicKey,
      protocol,
      user: buyerPda,
      vaultAuthority,
      vaultToken: usdcVault,
      destination: parentUsdc,
      tokenProgram: TOKEN_PROGRAM_ID,
    })
    .instruction(),
  "claim hijack with victim UserState",
);

await expectFailure(
  connection,
  buyer,
  await program.methods
    .claim()
    .accounts({
      wallet: buyer.publicKey,
      protocol,
      user: buyerPda,
      vaultAuthority,
      vaultToken: usdcVault,
      destination: parentUsdc,
      tokenProgram: TOKEN_PROGRAM_ID,
    })
    .instruction(),
  "claim to non-canonical destination",
);

let buyerAfterNegatives = await userState(buyerPda);
invariant(asBigInt(buyerAfterNegatives.lifetimeClaimedUsdc) === lifetimeBeforeNegatives, "failed claim changed lifetimeClaimed");
invariant(asBigInt(buyerAfterNegatives.selfAccruedUsdc) === 5_000_000n, "failed claim cleared SELF");
invariant(await tokenAmount(buyerUsdc) === buyerBalanceBeforeNegatives, "failed claim changed buyer balance");
invariant(await tokenAmount(parentUsdc) === parentBalanceBeforeNegatives, "failed claim changed attacker destination");
invariant(await tokenAmount(usdcVault) === vaultBeforeNegatives, "failed claim changed vault");

const validClaimIx = await program.methods
  .claim()
  .accounts({
    wallet: buyer.publicKey,
    protocol,
    user: buyerPda,
    vaultAuthority,
    vaultToken: usdcVault,
    destination: buyerUsdc,
    tokenProgram: TOKEN_PROGRAM_ID,
  })
  .instruction();
await send(connection, buyer, validClaimIx, "valid security buyer claim");

const buyerAfterClaim = await userState(buyerPda);
invariant(asBigInt(buyerAfterClaim.selfAccruedUsdc) === 0n, "valid claim did not clear SELF");
invariant(asBigInt(buyerAfterClaim.networkClaimableUsdc) === 0n, "valid claim did not clear network claimable");
invariant(asBigInt(buyerAfterClaim.lifetimeClaimedUsdc) - lifetimeBeforeNegatives === 5_000_000n, "valid claim lifetime delta mismatch");
invariant(await tokenAmount(buyerUsdc) === 5_000_000n, "valid claim buyer payout must be 5 USDC");
invariant(await tokenAmount(usdcVault) - vaultBeforeSpoof === 200_000n, "post-claim vault must retain exactly 0.2 USDC Pioneer liability");

const stateBeforeDoubleClaim = await userState(buyerPda);
const buyerBeforeDoubleClaim = await tokenAmount(buyerUsdc);
const vaultBeforeDoubleClaim = await tokenAmount(usdcVault);
await expectFailure(connection, buyer, validClaimIx, "double claim / replay");
const stateAfterDoubleClaim = await userState(buyerPda);
invariant(asBigInt(stateAfterDoubleClaim.lifetimeClaimedUsdc) === asBigInt(stateBeforeDoubleClaim.lifetimeClaimedUsdc), "double claim changed lifetimeClaimed");
invariant(asBigInt(stateAfterDoubleClaim.selfAccruedUsdc) === 0n, "double claim recreated SELF");
invariant(await tokenAmount(buyerUsdc) === buyerBeforeDoubleClaim, "double claim paid buyer again");
invariant(await tokenAmount(usdcVault) === vaultBeforeDoubleClaim, "double claim changed vault");
invariant(await tokenAmount(fakeTreasuryUsdc) === 0n, "fake Treasury received value");

console.log("DEVNET SECURITY ADVERSARIAL VALIDATION: PASS");
console.log(JSON.stringify({
  programId: programId.toBase58(),
  initialRealUsers: initialRealUsers.toString(),
  finalRealUsers: asBigInt((await protocolState()).realUserCount).toString(),
  validPurchaseUnits: "10",
  buyerClaimedUsdcAtoms: asBigInt((await userState(buyerPda)).lifetimeClaimedUsdc).toString(),
  residualPioneerVaultUsdcAtoms: (await tokenAmount(usdcVault)).toString(),
  fakeTreasuryUsdcAtoms: (await tokenAmount(fakeTreasuryUsdc)).toString(),
}, null, 2));
