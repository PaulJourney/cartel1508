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
import { getMint, TOKEN_PROGRAM_ID } from "@solana/spl-token";

const RPC_URL = process.env.ANCHOR_PROVIDER_URL || "http://127.0.0.1:8899";
const IDL_PATH = process.env.PROTOCOL_IDL || "target/idl/service_referral_protocol.json";
const WALLET_PATH = (process.env.ANCHOR_WALLET || "~/.config/solana/id.json").replace(
  /^~(?=$|\/)/,
  os.homedir(),
);

const FINAL_PROGRAM_ID = new PublicKey("DA214e5LFbj1WARXzu89k295azhKCGMFcVicsm7CsfpL");
const MAINNET_USDT_MINT = new PublicKey("Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB");
const MAINNET_USDC_MINT = new PublicKey("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v");
const MAINNET_SERVICE_TREASURY = new PublicKey("AepYo8xanmKuRiLVeYQuCTJoQr1nyKiTApoKwHMEg8fn");
const MAINNET_REGISTRATION_OPEN_AT = 1_788_238_800;
const ZERO_PUBKEY = new PublicKey("11111111111111111111111111111111");

function invariant(condition, message) {
  if (!condition) throw new Error(`PRODUCTION RUNTIME INVARIANT FAILED: ${message}`);
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
  const time = await connection.getBlockTime(slot);
  return time ?? Math.floor(Date.now() / 1000);
}

async function send(connection, signer, instruction, label) {
  const signature = await sendAndConfirmTransaction(
    connection,
    new Transaction().add(instruction),
    [signer],
    { commitment: "confirmed", preflightCommitment: "confirmed" },
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
      { commitment: "confirmed", preflightCommitment: "confirmed" },
    );
  } catch (error) {
    console.log(`PASS reject | ${label}: ${String(error.message || error).slice(0, 260)}`);
    return;
  }
  throw new Error(`PRODUCTION RUNTIME INVARIANT FAILED: expected rejection: ${label}`);
}

const payer = readKeypair(path.resolve(WALLET_PATH));
const idl = JSON.parse(fs.readFileSync(path.resolve(IDL_PATH), "utf8"));
const idlProgramId = new PublicKey(idl.address);
const connection = new Connection(RPC_URL, "confirmed");
const program = new Program(idl, { connection });

invariant(idlProgramId.equals(FINAL_PROGRAM_ID), "IDL is not bound to final Mainnet Program ID");

const programAccount = await connection.getAccountInfo(FINAL_PROGRAM_ID, "confirmed");
invariant(programAccount !== null, "final Program ID is absent from local validator");
invariant(programAccount.executable === true, "final Program ID is not executable");

const usdtMint = await getMint(connection, MAINNET_USDT_MINT, "confirmed", TOKEN_PROGRAM_ID);
const usdcMint = await getMint(connection, MAINNET_USDC_MINT, "confirmed", TOKEN_PROGRAM_ID);
invariant(usdtMint.decimals === 6, "canonical Mainnet USDT does not expose 6 decimals");
invariant(usdcMint.decimals === 6, "canonical Mainnet USDC does not expose 6 decimals");

console.log(`RPC:                 ${RPC_URL}`);
console.log(`Program ID:          ${FINAL_PROGRAM_ID.toBase58()}`);
console.log(`USDT mint:           ${MAINNET_USDT_MINT.toBase58()}`);
console.log(`USDC mint:           ${MAINNET_USDC_MINT.toBase58()}`);
console.log(`Service treasury:    ${MAINNET_SERVICE_TREASURY.toBase58()}`);
console.log(`Registration opens:  ${MAINNET_REGISTRATION_OPEN_AT}`);

const [protocol] = PublicKey.findProgramAddressSync([Buffer.from("protocol")], FINAL_PROGRAM_ID);
const [vaultAuthority] = PublicKey.findProgramAddressSync(
  [Buffer.from("vault-authority")],
  FINAL_PROGRAM_ID,
);
const [technicalRoot] = PublicKey.findProgramAddressSync(
  [Buffer.from("user"), ZERO_PUBKEY.toBuffer()],
  FINAL_PROGRAM_ID,
);

function initializeInstruction(serviceTreasury, usdt, usdc, openAt) {
  return program.methods
    .initialize(new BN(openAt))
    .accounts({
      initializer: payer.publicKey,
      serviceTreasury,
      usdtMint: usdt,
      usdcMint: usdc,
      protocol,
      vaultAuthority,
      technicalRoot,
      systemProgram: SystemProgram.programId,
    })
    .instruction();
}

await expectFailure(
  connection,
  payer,
  await initializeInstruction(
    Keypair.generate().publicKey,
    MAINNET_USDT_MINT,
    MAINNET_USDC_MINT,
    MAINNET_REGISTRATION_OPEN_AT,
  ),
  "reject non-production treasury",
);
invariant((await connection.getAccountInfo(protocol, "confirmed")) === null, "failed init created protocol state");
invariant((await connection.getAccountInfo(technicalRoot, "confirmed")) === null, "failed init created technical root");

await expectFailure(
  connection,
  payer,
  await initializeInstruction(
    MAINNET_SERVICE_TREASURY,
    MAINNET_USDT_MINT,
    MAINNET_USDC_MINT,
    MAINNET_REGISTRATION_OPEN_AT + 1,
  ),
  "reject non-frozen registration timestamp",
);
invariant((await connection.getAccountInfo(protocol, "confirmed")) === null, "wrong timestamp initialized protocol");

await expectFailure(
  connection,
  payer,
  await initializeInstruction(
    MAINNET_SERVICE_TREASURY,
    MAINNET_USDC_MINT,
    MAINNET_USDT_MINT,
    MAINNET_REGISTRATION_OPEN_AT,
  ),
  "reject swapped production mint roles",
);
invariant((await connection.getAccountInfo(protocol, "confirmed")) === null, "swapped mints initialized protocol");

const nowBeforeInit = await chainUnixTime(connection);
invariant(
  nowBeforeInit < MAINNET_REGISTRATION_OPEN_AT,
  `validator clock ${nowBeforeInit} is not before frozen launch ${MAINNET_REGISTRATION_OPEN_AT}`,
);

await send(
  connection,
  payer,
  await initializeInstruction(
    MAINNET_SERVICE_TREASURY,
    MAINNET_USDT_MINT,
    MAINNET_USDC_MINT,
    MAINNET_REGISTRATION_OPEN_AT,
  ),
  "initialize exact production configuration",
);

const state = await program.account.protocolState.fetch(protocol);
invariant(state.serviceTreasury.equals(MAINNET_SERVICE_TREASURY), "stored treasury mismatch");
invariant(state.usdtMint.equals(MAINNET_USDT_MINT), "stored USDT mint mismatch");
invariant(state.usdcMint.equals(MAINNET_USDC_MINT), "stored USDC mint mismatch");
invariant(
  asBigInt(state.registrationOpenAt) === BigInt(MAINNET_REGISTRATION_OPEN_AT),
  "stored registration timestamp mismatch",
);
invariant(asBigInt(state.pioneerPositionsAssigned) === 0n, "Pioneer positions are nonzero at genesis");
invariant(asBigInt(state.realUserCount) === 0n, "real user count is nonzero at genesis");
invariant(asBigInt(state.nextUnitId) === 1n, "next unit ID is not 1 at genesis");

const rootState = await program.account.userState.fetch(technicalRoot);
invariant(rootState.wallet.equals(ZERO_PUBKEY), "technical root wallet is not zero pubkey");
invariant(rootState.referrer.equals(ZERO_PUBKEY), "technical root referrer is not zero pubkey");

const preOpenUser = Keypair.generate();
await send(
  connection,
  payer,
  SystemProgram.transfer({
    fromPubkey: payer.publicKey,
    toPubkey: preOpenUser.publicKey,
    lamports: Math.floor(0.1 * LAMPORTS_PER_SOL),
  }),
  "fund pre-open registration signer locally",
);
const [preOpenUserPda] = PublicKey.findProgramAddressSync(
  [Buffer.from("user"), preOpenUser.publicKey.toBuffer()],
  FINAL_PROGRAM_ID,
);

await expectFailure(
  connection,
  preOpenUser,
  await program.methods
    .register()
    .accounts({
      wallet: preOpenUser.publicKey,
      protocol,
      referrerWallet: ZERO_PUBKEY,
      referrer: technicalRoot,
      user: preOpenUserPda,
      systemProgram: SystemProgram.programId,
    })
    .instruction(),
  "registration remains locked before frozen launch",
);
invariant(
  (await connection.getAccountInfo(preOpenUserPda, "confirmed")) === null,
  "rejected pre-open registration left a user PDA",
);

const stateAfterRejection = await program.account.protocolState.fetch(protocol);
invariant(asBigInt(stateAfterRejection.realUserCount) === 0n, "rejected registration changed real user count");
invariant(asBigInt(stateAfterRejection.nextUnitId) === 1n, "rejected registration changed next unit ID");

console.log("PASS exact Program ID executable");
console.log("PASS canonical Mainnet USDT/USDC accounts with 6 decimals");
console.log("PASS frozen treasury/mint/timestamp rejection checks");
console.log("PASS exact production initialization and genesis state");
console.log("PASS pre-open registration lock");
console.log("PRODUCTION RUNTIME EXACT-IDENTITY VALIDATION: PASS");
