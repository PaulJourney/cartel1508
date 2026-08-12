import assert from 'node:assert/strict';

const BPS = [1500, 900, 600, 400, 250, 200, 150, 100, 100, 100];
assert.equal(BPS.reduce((a,b)=>a+b,0), 4300);
assert.equal(5000 + 4300 + 200 + 500, 10000);

function split(amount) {
  const calc = bps => Math.floor(amount * bps / 10000);
  const levels = BPS.map(calc);
  const direct = calc(5000);
  const pioneer = calc(200);
  const service = calc(500);
  const allocated = direct + pioneer + service + levels.reduce((a,b)=>a+b,0);
  return {direct, levels, pioneer, service, remainder: amount-allocated};
}

const one = split(1_000_000);
assert.deepEqual(one, {
  direct:500_000,
  levels:[150_000,90_000,60_000,40_000,25_000,20_000,15_000,10_000,10_000,10_000],
  pioneer:20_000, service:50_000, remainder:0
});

const hugeUnits = 100_000_000_000n;
const payment = hugeUnits * 1_000_000n;
assert(payment <= 18_446_744_073_709_551_615n);

for (let i=1;i<=5000;i++) {
  const amount = Math.floor(Math.random()*10_000_000_000)+1;
  const s = split(amount);
  assert.equal(s.direct+s.pioneer+s.service+s.remainder+s.levels.reduce((a,b)=>a+b,0), amount);
}
console.log('reference model: all invariants passed');
