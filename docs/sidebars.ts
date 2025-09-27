import type {SidebarsConfig} from '@docusaurus/plugin-content-docs';

// This runs in Node.js - Don't use client-side code here (browser APIs, JSX...)

/**
 * Creating a sidebar enables you to:
 - create an ordered group of docs
 - render a sidebar for each doc of that group
 - provide next/previous navigation

 The sidebars can be generated from the filesystem, or explicitly defined here.

 Create as many sidebars as you want.
 */
const sidebars: SidebarsConfig = {
  tutorialSidebar: [
    {
      type: 'category',
      label: 'Concepts',
      items: [
        {
          type: 'category',
          label: 'Boards',
          items: ['concepts/boards', 'core-model', 'custom-geometry', '3d-boards'],
        },
        {
          type: 'category',
          label: 'Tiles',
          items: ['concepts/tiles', 'stacking-and-multi-char', 'stacking-rules', 'emoji-and-graphemes', 'language-packs'],
        },
        {
          type: 'category',
          label: 'Rules',
          items: ['concepts/rules', 'scoring-and-validation', 'persistence-and-replays'],
        },
        {
          type: 'category',
          label: 'Lexica',
          items: ['concepts/lexica', 'dictionary-engines', 'lexicon-integration'],
        },
        {
          type: 'category',
          label: 'Move Generation',
          items: ['concepts/move-generation', 'move-generation', 'animated-move-breakdown'],
        },
        {
          type: 'category',
          label: 'AI',
          items: ['concepts/ai', 'ai-overview', 'ai-framework'],
        },
      ],
    },
    {
      type: 'category',
      label: 'How-tos',
      items: [
        'how-tos/create-variant',
        'how-tos/add-emoji-language',
        'how-tos/enable-3d',
        'how-tos/write-plugin',
      ],
    },
    {
      type: 'category',
      label: 'Reference',
      items: [
        'architecture-overview',
        'getting-started',
        'install',
        {
          type: 'category',
          label: 'Bindings',
          items: ['c-abi', 'python', 'web-wasm-api', 'unity-integration', 'godot-integration'],
        },
        'performance',
      ],
    },
    {
      type: 'category',
      label: 'Playgrounds & Demos',
      items: ['playground', 'godot-web', 'classic-demo', 'python-demo', 'showcase'],
    },
    {
      type: 'category',
      label: 'Cookbook',
      items: ['cookbook', 'cookbook-binding-ai'],
    },
  ],
};

export default sidebars;
