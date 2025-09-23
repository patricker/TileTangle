import {execSync} from 'node:child_process';
import path from 'node:path';
import {themes as prismThemes} from 'prism-react-renderer';
import type {Config, Plugin} from '@docusaurus/types';
import type * as Preset from '@docusaurus/preset-classic';

// This runs in Node.js - Don't use client-side code here (browser APIs, JSX...)

const wasmBuildPlugin = (): Plugin<void> => ({
  name: 'tiletangle-wasm-build',
  async loadContent() {
    if (process.env.NODE_ENV !== 'production') {
      return;
    }

    if (process.env.TILETANGLE_SKIP_WASM_REBUILD === '1') {
      console.log('[wasm-build] Skipping WASM rebuild (env opt-out).');
      return;
    }

    console.log('[wasm-build] Rebuilding WASM artifacts before docs build...');
    execSync('npm run wasm:build', {
      cwd: path.resolve(__dirname),
      stdio: 'inherit',
    });
  },
});

const config: Config = {
  title: 'TileTangle',
  tagline: 'Universal word‑game engine',
  favicon: 'img/tiletangle_t.png',

  // Future flags, see https://docusaurus.io/docs/api/docusaurus-config#future
  future: {
    v4: true, // Improve compatibility with the upcoming Docusaurus v4
  },

  // Set the production url of your site here
  url: 'https://patricker.github.io',
  // Set the /<baseUrl>/ pathname under which your site is served
  // For GitHub Pages project sites, it is '/<projectName>/'
  baseUrl: '/',

  // GitHub pages deployment config.
  // If you aren't using GitHub pages, you don't need these.
  organizationName: 'patricker',
  projectName: 'TileTangle',

  onBrokenLinks: 'throw',
  onBrokenMarkdownLinks: 'warn',

  // Even if you don't use internationalization, you can use this field to set
  // useful metadata like html lang. For example, if your site is Chinese, you
  // may want to replace "en" with "zh-Hans".
  i18n: {
    defaultLocale: 'en',
    locales: ['en'],
  },

  presets: [
    [
      'classic',
      {
        docs: {
          sidebarPath: './sidebars.ts',
          editUrl: undefined,
        },
        blog: false,
        theme: {
          customCss: './src/css/custom.css',
        },
      } satisfies Preset.Options,
    ],
  ],
  plugins: [wasmBuildPlugin],

  themeConfig: {
    // Replace with your project's social card
    image: 'img/docusaurus-social-card.jpg',
    navbar: {
      title: 'Tile Tangle',
      logo: {
        alt: 'Tile Tangle Logo',
        src: 'img/tiletangle_t.png',
      },
      items: [
        { type: 'docSidebar', sidebarId: 'tutorialSidebar', position: 'left', label: 'Docs' },
        { to: '/docs/playground', label: 'Playground', position: 'left' },
        {
          href: 'https://github.com/patricker/TileTangle',
          label: 'GitHub',
          position: 'right',
        },
      ],
    },
    footer: {
      style: 'dark',
      links: [],
      copyright: `Docs content licensed under <a href="https://creativecommons.org/licenses/by/4.0/">Creative Commons Attribution 4.0 International (CC BY 4.0)</a>.`,
    },
    prism: {
      theme: prismThemes.github,
      darkTheme: prismThemes.dracula,
    },
  } satisfies Preset.ThemeConfig,
};

export default config;
