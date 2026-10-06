#!/usr/bin/env node
'use strict';

// Exercise the installed publisher's protocol using fixtures, without credentials or network calls.
const assert = require('node:assert/strict');
const path = require('node:path');
const { getOIDCCredential } = require(path.resolve(process.argv[2], 'out/oidc.js'));

const environment = {
  GITHUB_ACTIONS: 'true',
  ACTIONS_ID_TOKEN_REQUEST_URL: 'https://github.invalid/token?api-version=1',
  ACTIONS_ID_TOKEN_REQUEST_TOKEN: 'fixture-runtime-token',
};
function response(data, statusCode = 200) {
  return { statusCode, statusMessage: statusCode === 200 ? 'OK' : 'Forbidden',
    readBody: async () => JSON.stringify(data) };
}

async function main() {
  for (const marketplaceUrl of ['https://marketplace.visualstudio.com', 'https://marketplace.visualstudio.com/']) {
    const requests = [];
    const credential = await getOIDCCredential('Hyacine', {
      environment, marketplaceUrl,
      request: async (url, init) => {
        requests.push({ url, init });
        return requests.length === 1 ? response({ value: 'fixture-oidc-token' }) :
          response({ credential: 'fixture-session-token' });
      },
    });
    assert.equal(credential, 'fixture-session-token');
    assert.equal(requests.length, 2);
    const issuer = new URL(requests[0].url);
    assert.equal(issuer.searchParams.get('audience'), 'marketplace.visualstudio.com');
    assert.equal(issuer.searchParams.get('api-version'), '1');
    assert.equal(requests[0].init.headers.Authorization, 'Bearer fixture-runtime-token');
    assert.equal(requests[1].url, 'https://marketplace.visualstudio.com/_apis/gallery/token?api-version=7.2-preview.1');
    assert.equal(requests[1].init.method, 'POST');
    assert.equal(requests[1].init.headers.Authorization, 'FederatedToken fixture-oidc-token');
    assert.deepEqual(JSON.parse(requests[1].init.body), { publisherName: 'Hyacine' });
  }
  let calls = 0;
  await assert.rejects(getOIDCCredential('Hyacine', {
    environment,
    request: async () => ++calls === 1 ? response({ value: 'fixture-oidc-token' }) :
      response({ message: 'Trusted publishing policy is missing' }, 403),
  }), /403 Forbidden: Trusted publishing policy is missing/);
  assert.equal(calls, 2);
  await assert.rejects(getOIDCCredential('Hyacine', {
    environment: { GITHUB_ACTIONS: 'true' },
    request: async () => { throw new Error('Must not request a token without OIDC permissions'); },
  }), /permissions: id-token: write/);
  console.log('Verified Marketplace OIDC API version, authentication scheme and rejected credentials.');
}

main().catch(error => { console.error(error.message); process.exitCode = 1; });
