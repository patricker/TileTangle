import { test, expect } from '@playwright/test';

test('homepage loads', async ({ page }) => {
  await page.goto('/');
  await expect(page).toHaveTitle(/TileTangle/i);
  await expect(page.locator('nav.navbar')).toBeVisible();
});

test('AI Overview doc renders', async ({ page }) => {
  await page.goto('/docs/ai-overview');
  await expect(page.locator('article h1')).toContainText('AI Overview');
  await expect(page.locator('nav.navbar')).toBeVisible();
});

test('Persistence guide renders Playground controls', async ({ page }) => {
  await page.goto('/docs/persistence-and-replays');
  await expect(page.locator('article h1')).toContainText('Persistence & Replays');
  await expect(page.locator('article')).toContainText('snapshot_json');
});

test('Performance toolkit page renders', async ({ page }) => {
  await page.goto('/docs/performance');
  await expect(page.locator('article h1')).toContainText('Performance Toolkit');
});

test('playground supports undo/redo', async ({ page }) => {
  await page.goto('/docs/playground');
  await page.waitForSelector('text=Loading WASM…', { state: 'detached', timeout: 15000 }).catch(() => {});
  await page.waitForFunction(() => document.querySelectorAll('[data-testid="playground-board-cell"]').length > 0, { timeout: 15000 });
  await page.getByRole('button', { name: 'Show legal moves' }).click();
  const playFirst = page.locator('button', { hasText: /^Play$/ }).first();
  await playFirst.waitFor({ timeout: 10000 });
  await playFirst.click();
  const snapshotArea = page.locator('textarea[placeholder="Click Save Snapshot to capture the current game state"]');
  const parseSnapshot = (json: string) => JSON.parse(json) as { turn_num: number; players: Array<{ score: number }>; };
  await page.getByRole('button', { name: 'Save Snapshot' }).click();
  const firstSnapshot = await snapshotArea.inputValue();
  const first = parseSnapshot(firstSnapshot);
  await page.locator('[data-testid="playground-undo"]').click();
  await page.getByRole('button', { name: 'Save Snapshot' }).click();
  const secondSnapshot = await snapshotArea.inputValue();
  const second = parseSnapshot(secondSnapshot);
  expect(second.turn_num).toBeLessThan(first.turn_num);
  await page.locator('[data-testid="playground-redo"]').click();
  await page.getByRole('button', { name: 'Save Snapshot' }).click();
  const thirdSnapshot = await snapshotArea.inputValue();
  const third = parseSnapshot(thirdSnapshot);
  expect(third.turn_num).toEqual(first.turn_num);
  expect(third.players.map(p => p.score)).toEqual(first.players.map(p => p.score));
});
