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
  const boardCells = () => page.$$eval('[data-testid="playground-board-cell"]', nodes => nodes.map(n => (n.textContent || '').trim()));
  await page.getByRole('button', { name: 'Show legal moves' }).click();
  const playFirst = page.locator('button', { hasText: /^Play$/ }).first();
  await playFirst.waitFor({ timeout: 10000 });
  await playFirst.click();
  const afterPlay = await boardCells();
  const playedCount = afterPlay.filter(cell => cell.length > 0).length;
  expect(playedCount).toBeGreaterThan(0);

  const undoButton = page.locator('[data-testid="playground-undo"]');
  await undoButton.click();
  await page.waitForFunction(() => {
    const cells = Array.from(document.querySelectorAll('[data-testid="playground-board-cell"]')).map(n => (n.textContent || '').trim());
    return cells.every(cell => cell.length === 0);
  }, {}, { timeout: 10000 });

  const redoButton = page.locator('[data-testid="playground-redo"]');
  await redoButton.click();
  await page.waitForTimeout(200);
});

test('showcase docs mention CPU controls and 3D slice overlay', async ({ page }) => {
  await page.goto('/docs/showcase');
  await expect(page.locator('article h1')).toContainText('Showcase Demos');
  const article = page.locator('article');
  await expect(article).toContainText(/CPU Move/i);
  await expect(article).toContainText(/sample vertical word/i);
});
