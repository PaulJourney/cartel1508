import crypto from "node:crypto";
import fs from "node:fs";
import {
  AccountRole,
  address,
  appendTransactionMessageInstruction,
  createTransactionMessage,
  generateKeyPairSigner,
  lamports,
  pipe,
  setTransactionMessageFeePayerSigner,
  signTransactionMessageWithSigners,
} from "@solana/kit";
import { PublicKey } from "@solana/web3.js";
import { Clock, FailedTransactionMetadata, LiteSVM } from "litesvm";

const FROZEN_SOURCE_SHA = "36420887aad96b3c5b9680a35c6dc211b893c2bc";
const EXPECTED_SO_SHA256 = "0c632adebe065b56c5697d0ee9879d93977d10b55e3d554a17a4641ee1533369";
const SO_PATH = process.env.PRODUCTION_SO || "/tmp/verifiable/target/verifiable/service_referral_protocol.so";

const FINAL_PROGRAM_ID = "DA214e5LFbj1WARXzu89k295azhKCGMFcVicsm7CsfpL";
const MAINNET_USDT_MINT = "Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB";
const MAINNET_USDC_MINT = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";
const MAINNET_SERVICE_TREASURY = "AepYo8xanmKuRiLVeYQuCTJoQr1nyKiTApoKwHMEg8fn";
const MAINNET_REGISTRATION_OPEN_AT = 1_788_238_800n;
const ZERO_PUBKEY = "11111111111111111111111111111111";
const SYSTEM_PROGRAM_ID = "11111111111111111111111111111111";
const TOKEN_PROGRAM_ID = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA";
const ASSOCIATED_TOKEN_PROGRAM_ID = "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL";
const TOKEN_SCALE = 1_000_000n;

function invariant(condition, message) {
  if (!condition) throw new Error(`POST-OPEN PRODUCTION INVARIANT FAILED: ${message}`);
}

function sha256File(filename) {
  return crypto.createHash("sha256").update(fs.readFileSync(filename)).digest("hex");
}

function anchorDiscriminator(name) {
  return crypto.createHash("sha256").update(`global:${name}`).digest().subarray(0, 8);
}

function encodeI64(value) {
  const out = Buffer.alloc(8);
  out.writeBigInt64LE(BigInt(value));
  return out;
}

function encodeU64(value) {
  const out = Buffer.alloc(8);
  out.writeBigUInt64LE(BigInt(value));
  return out;
}

function ixData(name, value = null, signed = false) {
  if (value === null) return new Uint8Array(anchorDiscriminator(name));
  return new Uint8Array(Buffer.concat([
    anchorDiscriminator(name),
    signed ? encodeI64(value) : encodeU64(value),
  ]));
}

function pk(value) {
  return new PublicKey(value);
}

function kitAddress(value) {
  return address(typeof value === "string" ? value : value.toBase58());
}

function pda(seeds) {
  return PublicKey.findProgramAddressSync(seeds, pk(FINAL_PROGRAM_ID))[0];
}

function ata(owner, mint) {
  return PublicKey.findProgramAddressSync(
    [pk(owner).toBuffer(), pk(TOKEN_PROGRAM_ID).toBuffer(), pk(mint).toBuffer()],
    pk(ASSOCIATED_TOKEN_PROGRAM_ID),
  )[0];
}

function pubkeyBytes(value) {
  return pk(value).toBuffer();
}

function mintData(authority) {
  const data = Buffer.alloc(82);
  data.writeUInt32LE(1, 0);
  pubkeyBytes(authority).copy(data, 4);
  data.writeBigUInt64LE(0n, 36);
  data[44] = 6;
  data[45] = 1;
  data.writeUInt32LE(0, 46);
  return new Uint8Array(data);
}

function tokenAccountData(mint, owner, amount) {
  const data = Buffer.alloc(165);
  pubkeyBytes(mint).copy(data, 0);
  pubkeyBytes(owner).copy(data, 32);
  data.writeBigUInt64LE(BigInt(amount), 64);
  data.writeUInt32LE(0, 72);
  data[108] = 1;
  data.writeUInt32LE(0, 109);
  data.writeBigUInt64LE(0n, 113);
  data.writeBigUInt64LE(0n, 121);
  data.writeUInt32LE(0, 129);
  return new Uint8Array(data);
}

function setRawAccount(svm, accountAddress, programAddress, data, executable = false) {
  const bytes = data instanceof Uint8Array ? data : new Uint8Array(data);
  const rent = svm.minimumBalanceForRentExemption(BigInt(bytes.length));
  svm.setAccount({
    address: kitAddress(accountAddress),
    data: bytes,
    executable,
    lamports: lamports(rent),
    programAddress: kitAddress(programAddress),
    space: BigInt(bytes.length),
  });
}

function setSystemAccount(svm, accountAddress) {
  svm.setAccount({
    address: kitAddress(accountAddress),
    data: new Uint8Array(),
    executable: false,
    lamports: lamports(1_000_000n),
    programAddress: kitAddress(SYSTEM_PROGRAM_ID),
    space: 0n,
  });
}

function accountData(svm, accountAddress) {
  const account = svm.getAccount(kitAddress(accountAddress));
  if (account === null || account === undefined || account.exists === false) {
    throw new Error(`missing account ${accountAddress}`);
  }
  return account.data;
}

function tokenBalance(svm, accountAddress) {
  const data = Buffer.from(accountData(svm, accountAddress));
  invariant(data.length === 165, `token account ${accountAddress} has data length ${data.length}`);
  return data.readBigUInt64LE(64);
}

function meta(accountAddress, role, signer = null) {
  const out = { address: kitAddress(accountAddress), role };
  if (signer) out.signer = signer;
  return out;
}

function protocolInstruction(name, accounts, value = null, signed = false) {
  return {
    programAddress: kitAddress(FINAL_PROGRAM_ID),
    accounts,
    data: ixData(name, value, signed),
  };
}

async function sendInstruction(svm, feePayer, instruction, label, expectFailure = false) {
  const message = pipe(
    createTransactionMessage({ version: 0 }),
    (tx) => setTransactionMessageFeePayerSigner(feePayer, tx),
    (tx) => svm.setTransactionMessageLifetimeUsingLatestBlockhash(tx),
    (tx) => appendTransactionMessageInstruction(instruction, tx),
  );
  const transaction = await signTransactionMessageWithSigners(message);
  const result = svm.sendTransaction(transaction);
  const failed = result instanceof FailedTransactionMetadata;
  if (expectFailure) {
    invariant(failed, `${label} unexpectedly succeeded`);
    console.log(`PASS reject | ${label}: ${String(result.err()).slice(0, 300)}`);
    return result;
  }
  if (failed) {
    throw new Error(`${label} failed: ${String(result.err())}\n${result.logs().join("\n")}`);
  }
  console.log(`PASS tx | ${label}`);
  return result;
}

const actualSoHash = sha256File(SO_PATH);
invariant(actualSoHash === EXPECTED_SO_SHA256, `production .so hash mismatch: ${actualSoHash}`);

const svm = new LiteSVM().withDefaultPrograms();
svm.addProgramFromFile(kitAddress(FINAL_PROGRAM_ID), SO_PATH);

const payer = await generateKeyPairSigner();
const user = await generateKeyPairSigner();
svm.airdrop(payer.address, lamports(20_000_000_000n));
svm.airdrop(user.address, lamports(5_000_000_000n));
setSystemAccount(svm, MAINNET_SERVICE_TREASURY);
setRawAccount(svm, MAINNET_USDT_MINT, TOKEN_PROGRAM_ID, mintData(payer.address));
setRawAccount(svm, MAINNET_USDC_MINT, TOKEN_PROGRAM_ID, mintData(payer.address));

const protocol = pda([Buffer.from("protocol")]);
const vaultAuthority = pda([Buffer.from("vault-authority")]);
const technicalRoot = pda([Buffer.from("user"), pk(ZERO_PUBKEY).toBuffer()]);
const userPda = pda([Buffer.from("user"), pk(user.address).toBuffer()]);

const initialClock = svm.getClock();
initialClock.unixTimestamp = MAINNET_REGISTRATION_OPEN_AT - 100n;
initialClock.epochStartTimestamp = MAINNET_REGISTRATION_OPEN_AT - 100n;
svm.setClock(initialClock);

const initializeIx = protocolInstruction(
  "initialize",
  [
    meta(payer.address, AccountRole.WRITABLE_SIGNER, payer),
    meta(MAINNET_SERVICE_TREASURY, AccountRole.READONLY),
    meta(MAINNET_USDT_MINT, AccountRole.READONLY),
    meta(MAINNET_USDC_MINT, AccountRole.READONLY),
    meta(protocol, AccountRole.WRITABLE),
    meta(vaultAuthority, AccountRole.READONLY),
    meta(technicalRoot, AccountRole.WRITABLE),
    meta(SYSTEM_PROGRAM_ID, AccountRole.READONLY),
  ],
  MAINNET_REGISTRATION_OPEN_AT,
  true,
);
await sendInstruction(svm, payer, initializeIx, "initialize exact frozen production configuration");

const preOpenRegisterIx = protocolInstruction("register", [
  meta(user.address, AccountRole.WRITABLE_SIGNER, user),
  meta(protocol, AccountRole.WRITABLE),
  meta(ZERO_PUBKEY, AccountRole.READONLY),
  meta(technicalRoot, AccountRole.READONLY),
  meta(userPda, AccountRole.WRITABLE),
  meta(SYSTEM_PROGRAM_ID, AccountRole.READONLY),
]);
await sendInstruction(svm, user, preOpenRegisterIx, "registration before frozen opening", true);

const postOpenClock = svm.getClock();
postOpenClock.unixTimestamp = MAINNET_REGISTRATION_OPEN_AT + 1n;
svm.setClock(postOpenClock);
await sendInstruction(svm, user, preOpenRegisterIx, "registration after frozen opening");

const usdtVault = ata(vaultAuthority, MAINNET_USDT_MINT);
const usdcVault = ata(vaultAuthority, MAINNET_USDC_MINT);
const treasuryUsdt = ata(MAINNET_SERVICE_TREASURY, MAINNET_USDT_MINT);
const treasuryUsdc = ata(MAINNET_SERVICE_TREASURY, MAINNET_USDC_MINT);
const userUsdc = ata(user.address, MAINNET_USDC_MINT);

setRawAccount(svm, usdtVault, TOKEN_PROGRAM_ID, tokenAccountData(MAINNET_USDT_MINT, vaultAuthority, 0n));
setRawAccount(svm, usdcVault, TOKEN_PROGRAM_ID, tokenAccountData(MAINNET_USDC_MINT, vaultAuthority, 0n));
setRawAccount(svm, treasuryUsdt, TOKEN_PROGRAM_ID, tokenAccountData(MAINNET_USDT_MINT, MAINNET_SERVICE_TREASURY, 0n));
setRawAccount(svm, treasuryUsdc, TOKEN_PROGRAM_ID, tokenAccountData(MAINNET_USDC_MINT, MAINNET_SERVICE_TREASURY, 0n));
setRawAccount(svm, userUsdc, TOKEN_PROGRAM_ID, tokenAccountData(MAINNET_USDC_MINT, user.address, 10n * TOKEN_SCALE));

const purchaseIx = protocolInstruction(
  "purchase_and_distribute",
  [
    meta(user.address, AccountRole.WRITABLE_SIGNER, user),
    meta(protocol, AccountRole.WRITABLE),
    meta(userPda, AccountRole.WRITABLE),
    meta(userUsdc, AccountRole.WRITABLE),
    meta(vaultAuthority, AccountRole.READONLY),
    meta(usdtVault, AccountRole.WRITABLE),
    meta(usdcVault, AccountRole.WRITABLE),
    meta(treasuryUsdt, AccountRole.WRITABLE),
    meta(treasuryUsdc, AccountRole.WRITABLE),
    meta(technicalRoot, AccountRole.WRITABLE),
    meta(technicalRoot, AccountRole.WRITABLE),
    meta(technicalRoot, AccountRole.WRITABLE),
    meta(technicalRoot, AccountRole.WRITABLE),
    meta(technicalRoot, AccountRole.WRITABLE),
    meta(technicalRoot, AccountRole.WRITABLE),
    meta(technicalRoot, AccountRole.WRITABLE),
    meta(technicalRoot, AccountRole.WRITABLE),
    meta(technicalRoot, AccountRole.WRITABLE),
    meta(TOKEN_PROGRAM_ID, AccountRole.READONLY),
  ],
  10n,
  false,
);

invariant(tokenBalance(svm, userUsdc) === 10n * TOKEN_SCALE, "user source initial balance mismatch");
invariant(tokenBalance(svm, usdcVault) === 0n, "USDC vault initial balance mismatch");
invariant(tokenBalance(svm, treasuryUsdc) === 0n, "Treasury USDC initial balance mismatch");

await sendInstruction(svm, user, purchaseIx, "post-open 10-unit USDC purchase");
invariant(tokenBalance(svm, userUsdc) === 0n, "purchase did not debit exactly 10 USDC");
invariant(tokenBalance(svm, usdcVault) === 5n * TOKEN_SCALE, "purchase vault balance must be exactly 5 USDC SELF liability");
invariant(tokenBalance(svm, treasuryUsdc) === 5n * TOKEN_SCALE, "purchase Treasury balance must be exactly 5 USDC");
invariant(tokenBalance(svm, usdtVault) === 0n, "USDT vault changed during USDC purchase");
invariant(tokenBalance(svm, treasuryUsdt) === 0n, "USDT Treasury changed during USDC purchase");

const claimIx = protocolInstruction("claim", [
  meta(user.address, AccountRole.WRITABLE_SIGNER, user),
  meta(protocol, AccountRole.READONLY),
  meta(userPda, AccountRole.WRITABLE),
  meta(vaultAuthority, AccountRole.READONLY),
  meta(usdcVault, AccountRole.WRITABLE),
  meta(userUsdc, AccountRole.WRITABLE),
  meta(TOKEN_PROGRAM_ID, AccountRole.READONLY),
]);
await sendInstruction(svm, user, claimIx, "ACTIVE SELF claim on exact production runtime");
invariant(tokenBalance(svm, userUsdc) === 5n * TOKEN_SCALE, "claim did not pay exactly 5 USDC");
invariant(tokenBalance(svm, usdcVault) === 0n, "USDC vault must be zero after exact SELF claim");
invariant(tokenBalance(svm, treasuryUsdc) === 5n * TOKEN_SCALE, "claim mutated Treasury USDC");

await sendInstruction(svm, user, claimIx, "immediate production double-claim", true);
invariant(tokenBalance(svm, userUsdc) === 5n * TOKEN_SCALE, "failed double-claim changed user balance");
invariant(tokenBalance(svm, usdcVault) === 0n, "failed double-claim changed vault balance");
invariant(tokenBalance(svm, treasuryUsdc) === 5n * TOKEN_SCALE, "failed double-claim changed Treasury balance");

console.log(`FROZEN SOURCE SHA: ${FROZEN_SOURCE_SHA}`);
console.log(`PRODUCTION SO SHA256: ${actualSoHash}`);
console.log(`PROGRAM ID: ${FINAL_PROGRAM_ID}`);
console.log(`REGISTRATION OPEN AT: ${MAINNET_REGISTRATION_OPEN_AT}`);
console.log("PASS pre-open registration rejected");
console.log("PASS post-open registration succeeds on exact production Program ID");
console.log("PASS 10-unit USDC purchase exact balances: user=0 vault=5 treasury=5");
console.log("PASS ACTIVE claim exact balances: user=5 vault=0 treasury=5");
console.log("PASS immediate double-claim rejected with balances unchanged");
console.log("PRODUCTION POST-OPEN PURCHASE/CLAIM SUPPLEMENT: PASS");
