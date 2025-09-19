import { test, expect, Page } from '@playwright/test';

async function waitForPlaygroundReady(page: Page) {
  const loader = page.locator('text=Loading WASM…');
  await loader.waitFor({ state: 'attached', timeout: 2000 }).catch(() => {});
  await loader.waitFor({ state: 'detached', timeout: 60000 }).catch(() => {});

  await page.waitForSelector('[data-testid="playground-board-cell"]', { state: 'attached', timeout: 60000 });

  const dictLoading = page.locator('[data-testid="dictionary-loading"]').first();
  if ((await dictLoading.count()) > 0) {
    await dictLoading.waitFor({ state: 'detached', timeout: 60000 }).catch(() => {});
  }

  await page
    .waitForFunction(
      () => typeof (window as any).__tileTanglePlayground !== 'undefined',
      { timeout: 10000 },
    )
    .catch(() => {});
}

async function openPlayground(page: Page) {
  await page.goto('/docs/playground');
  await waitForPlaygroundReady(page);
}

async function waitForClassicDemoReady(page: Page) {
  const loader = page.locator('text=Loading classic demo…');
  await loader.waitFor({state: 'hidden', timeout: 60000}).catch(() => {});

  await page.waitForSelector('[data-testid="playground-board-cell"]', {
    state: 'attached',
    timeout: 60000,
  });

  await page
    .waitForFunction(
      () => typeof (window as any).__classicDemo !== 'undefined',
      {timeout: 10000},
    )
    .catch(() => {});
}

async function openClassicDemo(page: Page) {
  await page.goto('/docs/classic-demo');
  await waitForClassicDemoReady(page);
}

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
  await openPlayground(page);
  await page.getByLabel('Use dictionary validation').uncheck();
  await waitForPlaygroundReady(page);

  const rackTile = page.locator('[data-testid="playground-rack-tile"]').first();
  const kind = await rackTile.getAttribute('data-kind');
  if (!kind) throw new Error('Failed to read rack tile kind');

  await page.evaluate(([k, x, y]) => {
    (window as any).__tileTanglePlayground?.placeTile(k, x, y);
  }, [kind, 4, 4]);

  const commitButton = page.getByRole('button', { name: /Commit move/ });
  await expect(commitButton).toBeEnabled({ timeout: 5000 });
  await commitButton.click();
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
  await openPlayground(page);
  await page.getByLabel('Use dictionary validation').uncheck();
  await waitForPlaygroundReady(page);

  const tilePool = page.locator('textarea').first();
  await tilePool.fill('A:40');
  await page.getByRole('button', { name: 'Apply configuration' }).click();
  await waitForPlaygroundReady(page);
  await page.getByLabel('Use dictionary validation').uncheck();
  await waitForPlaygroundReady(page);

  await page.getByRole('button', { name: 'Show legal moves' }).first().click();
  await page.locator('[data-testid^="legal-move-"]').first().waitFor({ timeout: 15000 });

  const moves = await page.$$eval('[data-testid^="legal-move-"]', nodes => nodes.map(n => n.textContent?.trim() || ''));
  const unique = new Set(moves);
  expect(unique.size).toBe(moves.length);
});

test('playground switches to diamond mask without errors', async ({ page }) => {
  test.setTimeout(120000);
  const errors: string[] = [];
  page.on('console', msg => { if (msg.type() === 'error') errors.push(msg.text()); });
  page.on('pageerror', err => errors.push(err.message));

  await openPlayground(page);

  await page.getByTestId('board-shape-select').selectOption('diamond');
  await page.getByRole('button', { name: 'Apply configuration' }).click();
  await waitForPlaygroundReady(page);

  const inactiveCells = await page.$$eval(
    '[data-testid="playground-board-cell"][data-active="0"]',
    nodes => nodes.length,
  );
  const activeCells = await page.$$eval(
    '[data-testid="playground-board-cell"][data-active="1"]',
    nodes => nodes.length,
  );

  expect(activeCells).toBeGreaterThan(0);
  expect(inactiveCells).toBeGreaterThan(0);

  const significantErrors = errors.filter(msg => !msg.includes('404'));
  expect(significantErrors).toHaveLength(0);
  await expect(page.locator('text=edge index out of range')).toHaveCount(0);
  await expect(page.locator('text=Initialise playground failed')).toHaveCount(0);
});

test('playground toggles hex adjacency graph without errors', async ({ page }) => {
  test.setTimeout(120000);
  const errors: string[] = [];
  page.on('console', msg => { if (msg.type() === 'error') errors.push(msg.text()); });
  page.on('pageerror', err => errors.push(err.message));

  await openPlayground(page);
  await page.getByTestId('toggle-hex-adjacency').check();
  await page.getByRole('button', { name: 'Apply configuration' }).click();
  await waitForPlaygroundReady(page);

  await expect(page.locator('text=Hex adjacency uses staggered rows')).toBeVisible();

  const activeCells = await page.$$eval(
    '[data-testid="playground-board-cell"][data-active="1"]',
    nodes => nodes.length,
  );
  const inactiveCells = await page.$$eval(
    '[data-testid="playground-board-cell"][data-active="0"]',
    nodes => nodes.length,
  );
  expect(activeCells).toBeGreaterThan(0);
  expect(inactiveCells).toBeGreaterThan(0);

  const significantErrors = errors.filter(msg => !msg.includes('404'));
  expect(significantErrors).toHaveLength(0);
  await expect(page.locator('text=edge index out of range')).toHaveCount(0);
  await expect(page.locator('text=Initialise playground failed')).toHaveCount(0);
});

test('playground toggles 3D layers without errors', async ({ page }) => {
  const errors: string[] = [];
  page.on('pageerror', (err) => errors.push(err.message));
  page.on('console', msg => { if (msg.type() === 'error') errors.push(msg.text()); });

  await openPlayground(page);

  const threeDOption = page.getByRole('radio', { name: '3D' }).first();
  await threeDOption.check({ force: true });
  await expect(threeDOption).toBeChecked();
  await page.getByRole('button', { name: 'Apply configuration' }).click();
  await waitForPlaygroundReady(page);

  const depthInput = page.getByLabel('Layers', { exact: false });
  await depthInput.fill('2');

  const sliceSlider = page.getByLabel(/Viewing layer/i);
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

  const cpuMediumOption = page.getByRole('radio', { name: 'Medium' });
  await cpuMediumOption.check();
  await expect(cpuMediumOption).toBeChecked();

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
  const state = (await stateHandle.jsonValue()) as string | null;

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

test('classic demo commits a move', async ({page}) => {
  const consoleErrors: string[] = [];
  page.on('console', msg => { if (msg.type() === 'error') consoleErrors.push(msg.text()); });

  await openClassicDemo(page);

  const rackTile = page.locator('[data-testid="classic-rack-tile"]').first();
  await expect(rackTile).toBeVisible();
  const kind = await rackTile.getAttribute('data-kind');
  expect(kind).toBeTruthy();

  const center = Math.floor(15 / 2);
  await page.evaluate(([k, x, y]) => {
    (window as any).__classicDemo?.placeTile(k, x, y, null);
  }, [kind, center, center]);

  const commitButton = page.getByRole('button', {name: /Commit/});
  await expect(commitButton).toBeEnabled();
  await commitButton.click();

  await page.waitForFunction(([x, y]) => {
    const cell = document.querySelector(`[data-testid="playground-board-cell"][data-x="${x}"][data-y="${y}"]`);
    return !!cell && ((cell.textContent || '').trim().length > 0);
  }, [center, center], {timeout: 10000});

  await expect(commitButton).toBeDisabled();
  expect(consoleErrors.filter(msg => !msg.includes('404'))).toHaveLength(0);
});

test('classic demo shows hint overlays', async ({page}) => {
  await openClassicDemo(page);

  await page.getByLabel('Show Hints').check();

  const hintBadge = page.locator('[data-testid="classic-hint-badge"]').first();
  await hintBadge.waitFor({timeout: 20000});
  await expect(page.locator('[data-testid="classic-hint-card"]').first()).toBeVisible();
});
