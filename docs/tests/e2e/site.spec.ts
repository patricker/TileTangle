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
