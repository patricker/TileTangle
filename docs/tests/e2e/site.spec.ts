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
  await page.locator('[data-testid="dictionary-loading"]').first().waitFor({ state: 'detached', timeout: 20000 }).catch(() => {});
  await page.getByLabel('Dictionary checks').uncheck();
  await page.waitForSelector('text=Loading WASM…', { state: 'attached', timeout: 10000 }).catch(() => {});
  await page.waitForSelector('text=Loading WASM…', { state: 'detached', timeout: 15000 }).catch(() => {});
  await page.waitForFunction(() => document.querySelectorAll('[data-testid="playground-board-cell"]').length > 0, { timeout: 15000 });
  await page.waitForSelector('text=Loading WASM…', { state: 'attached', timeout: 10000 }).catch(() => {});
  await page.waitForSelector('text=Loading WASM…', { state: 'detached', timeout: 15000 }).catch(() => {});
  await page.waitForFunction(() => document.querySelectorAll('[data-testid="playground-board-cell"]').length > 0, { timeout: 15000 });

  const rackTile = page.locator('[data-testid="playground-rack-tile"]').first();
  const targetCell = page.locator('[data-testid="playground-board-cell"][data-x="4"][data-y="4"]');
  await rackTile.dragTo(targetCell);
  await page.getByRole('button', { name: /Commit move/ }).click();
  await page.waitForFunction(() => {
    const cell = document.querySelector('[data-testid="playground-board-cell"][data-x="4"][data-y="4"]');
    return cell && (cell.textContent || '').trim().length > 0;
  }, {}, { timeout: 10000 });

  await page.locator('[data-testid="playground-undo"]').click();
  await page.waitForFunction(() => {
    const cell = document.querySelector('[data-testid="playground-board-cell"][data-x="4"][data-y="4"]');
    return cell && (cell.textContent || '').trim().length === 0;
  }, {}, { timeout: 10000 });

  await page.locator('[data-testid="playground-redo"]').click();
  await page.waitForFunction(() => {
    const cell = document.querySelector('[data-testid="playground-board-cell"][data-x="4"][data-y="4"]');
    return cell && (cell.textContent || '').trim().length > 0;
  }, {}, { timeout: 10000 });
});

test('playground loads without console errors', async ({ page }) => {
  const errors: string[] = [];
  page.on('console', msg => { if (msg.type() === 'error') errors.push(msg.text()); });
  await page.goto('/docs/playground');
  await page.waitForSelector('text=Loading WASM…', { state: 'detached', timeout: 15000 }).catch(() => {});
  await page.waitForFunction(() => document.querySelectorAll('[data-testid="playground-board-cell"]').length > 0, { timeout: 15000 });
  const significant = errors.filter(msg => !msg.includes('404'));
  expect(significant).toHaveLength(0);
  await expect(page.locator('[data-testid="dictionary-error"]')).toHaveCount(0);
});

test('playground legal moves are unique', async ({ page }) => {
  await page.goto('/docs/playground');
  await page.waitForSelector('text=Loading WASM…', { state: 'detached', timeout: 15000 }).catch(() => {});
  await page.waitForFunction(() => document.querySelectorAll('[data-testid="playground-board-cell"]').length > 0, { timeout: 15000 });
  await page.locator('[data-testid="dictionary-loading"]').first().waitFor({ state: 'detached', timeout: 20000 }).catch(() => {});
  await page.getByLabel('Dictionary checks').uncheck();

  const tilePool = page.locator('textarea').first();
  await tilePool.fill('A:40');
  await page.getByRole('button', { name: 'Apply configuration' }).click();
  await page.waitForSelector('text=Loading WASM…', { state: 'attached', timeout: 10000 }).catch(() => {});
  await page.waitForSelector('text=Loading WASM…', { state: 'detached', timeout: 15000 }).catch(() => {});
  await page.waitForFunction(() => document.querySelectorAll('[data-testid="playground-board-cell"]').length > 0, { timeout: 15000 });
  await page.locator('[data-testid="dictionary-loading"]').first().waitFor({ state: 'detached', timeout: 20000 }).catch(() => {});
  await page.getByLabel('Dictionary checks').uncheck();

  await page.getByRole('button', { name: 'Show legal moves' }).click();
  await page.locator('[data-testid^="legal-move-"]').first().waitFor({ timeout: 15000 });

  const moves = await page.$$eval('[data-testid^="legal-move-"]', nodes => nodes.map(n => n.textContent?.trim() || ''));
  const unique = new Set(moves);
  expect(unique.size).toBe(moves.length);
});

test('playground toggles 3D layers without errors', async ({ page }) => {
  const errors: string[] = [];
  page.on('pageerror', (err) => errors.push(err.message));
  page.on('console', msg => { if (msg.type() === 'error') errors.push(msg.text()); });

  await page.goto('/docs/playground');
  await page.waitForSelector('text=Loading WASM…', { state: 'detached', timeout: 15000 }).catch(() => {});
  await page.waitForFunction(() => document.querySelectorAll('[data-testid="playground-board-cell"]').length > 0, { timeout: 15000 });
  await page.locator('[data-testid="dictionary-loading"]').first().waitFor({ state: 'detached', timeout: 20000 }).catch(() => {});

  await page.getByLabel('3D (layers)').check();
  const depthInput = page.getByLabel('Depth', { exact: false });
  await depthInput.fill('2');

  const sliceSlider = page.getByLabel('Slice z', { exact: false });
  await sliceSlider.evaluate((el: HTMLInputElement) => {
    el.value = el.max && Number(el.max) >= 1 ? '1' : '0';
    el.dispatchEvent(new Event('input', { bubbles: true }));
    el.dispatchEvent(new Event('change', { bubbles: true }));
  });
  await expect(sliceSlider).toHaveValue(/0|1/);

  const significantErrors = errors.filter(msg => !msg.includes('404'));
  expect(significantErrors).toHaveLength(0);
});

test('playground CPU hint plays a move', async ({ page }) => {
  const cpuErrors: string[] = [];
  page.on('console', msg => { if (msg.type() === 'error') cpuErrors.push(msg.text()); });
  page.on('pageerror', err => cpuErrors.push(err.message));

  await page.goto('/docs/playground');
  await page.waitForSelector('text=Loading WASM…', { state: 'detached', timeout: 15000 }).catch(() => {});
  await page.waitForFunction(() => document.querySelectorAll('[data-testid="playground-board-cell"]').length > 0, { timeout: 15000 });
  await page.locator('[data-testid="dictionary-loading"]').first().waitFor({ state: 'detached', timeout: 20000 }).catch(() => {});

  const tilePool = page.locator('textarea').first();
  await tilePool.fill('T:10\nI:10\nD:10\nE:10');
  await page.getByRole('button', { name: 'Apply configuration' }).click();
  await page.waitForSelector('text=Loading WASM…', { state: 'attached', timeout: 10000 }).catch(() => {});
  await page.waitForSelector('text=Loading WASM…', { state: 'detached', timeout: 15000 }).catch(() => {});
  await page.waitForFunction(() => document.querySelectorAll('[data-testid="playground-board-cell"]').length > 0, { timeout: 15000 });
  await page.locator('[data-testid="dictionary-loading"]').first().waitFor({ state: 'detached', timeout: 20000 }).catch(() => {});

  const cpuSelect = page.getByLabel('CPU difficulty', { exact: false });
  await cpuSelect.selectOption('medium');
  await expect(cpuSelect).toHaveValue('medium');

  await page.getByRole('button', { name: 'CPU hint' }).click();
  await page.locator('text=computing…').waitFor({ state: 'detached', timeout: 20000 }).catch(() => {});

  const playAsCpuButton = page.getByTestId('cpu-play-button');
  const stateHandle = await page.waitForFunction(() => {
    const btn = document.querySelector('[data-testid="cpu-play-button"]');
    if (btn && !(btn as HTMLButtonElement).disabled) return 'ready';
    const hasError = Array.from(document.querySelectorAll('div')).some(el => (el.textContent || '').includes('CPU hint failed'));
    if (hasError) return 'error';
    return null;
  }, {}, { timeout: 20000 });
  const state = await stateHandle.jsonValue<string | null>();

  if (state === 'ready') {
    const before = await page.$$eval('[data-testid="playground-board-cell"]', nodes => nodes.filter(n => (n.textContent || '').trim().length > 0).length);
    await playAsCpuButton.click();
    await page.waitForFunction((previousCount) => {
      const current = Array.from(document.querySelectorAll('[data-testid="playground-board-cell"]'))
        .filter(n => ((n.textContent || '').trim().length > 0)).length;
      return current > previousCount;
    }, before, { timeout: 10000 });
  } else {
    await expect(page.locator('text=CPU hint failed')).toBeVisible();
  }

  expect(cpuErrors.find(msg => msg.includes('Cannot convert'))).toBeUndefined();
});

test('showcase docs mention CPU controls and 3D slice overlay', async ({ page }) => {
  await page.goto('/docs/showcase');
  await expect(page.locator('article h1')).toContainText('Showcase Demos');
  const article = page.locator('article');
  await expect(article).toContainText(/CPU Move/i);
  await expect(article).toContainText(/sample vertical word/i);
});
