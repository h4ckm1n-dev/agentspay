const assert = require('node:assert/strict');
const { readFileSync } = require('node:fs');
const { test } = require('node:test');

test('security review keeps the patched Next minor and PostCSS override', () => {
  const app = require('../apps/frontend/package.json');
  assert.equal(app.dependencies.next, '15.5.28');
  assert.equal(app.devDependencies['eslint-config-next'], '15.5.28');
  assert.equal(require('../package.json').pnpm.overrides.postcss, '^8.5.23');
});

test('manual web review publishes only an immutable image with no deploy step', () => {
  const workflow = readFileSync('.github/workflows/ci.yml', 'utf8').split('  security-web:')[1];
  assert.match(workflow, /github.event_name == 'workflow_dispatch'/);
  assert.match(workflow, /agentspay-web:\$\{\{ github.sha \}\}/);
  assert.doesNotMatch(workflow, /:latest|kubectl|SABAICLOUD_GITOPS_TOKEN|h4ckm1n-dev\/Infra/);
});
