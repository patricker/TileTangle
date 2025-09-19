/// <reference types="@docusaurus/module-type-aliases" />

import type React from 'react';

declare module '/wasm/engine/pkg/tiletangle_wasm.js' {
  const init: (input?: RequestInfo | URL | Response | BufferSource | WebAssembly.Module) => Promise<any>;
  export default init;
  export const new_game: (...args: any[]) => any;
  export const set_reading_direction: (...args: any[]) => void;
  export const set_stacking: (...args: any[]) => void;
  export const set_free_word_mode: (...args: any[]) => void;
  export const set_dictionary_from_fst_bytes: (...args: any[]) => void;
  export const set_dictionary_from_text_engine: (...args: any[]) => void;
  export const set_dictionary_from_text: (...args: any[]) => void;
  export const set_rack: (...args: any[]) => void;
  export const get_board: (...args: any[]) => string;
  export const snapshot_state_json: (...args: any[]) => string;
  export const generate_moves: (...args: any[]) => string;
  export const play_move: (...args: any[]) => string;
  export const best_move: (...args: any[]) => string;
  export const get_event_log: (...args: any[]) => string;
  export const pass_turn: (...args: any[]) => void;
  export const exchange_tiles: (...args: any[]) => void;
  export const undo: (...args: any[]) => void;
  export const redo: (...args: any[]) => void;
  export const set_dictionary_from_text_engine_worker: (...args: any[]) => void;
}

declare module '*.module.css' {
  const classes: Record<string, string>;
  export default classes;
}

declare module '*.css' {
  const classes: Record<string, string>;
  export default classes;
}

declare module '@docusaurus/useBaseUrl' {
  export default function useBaseUrl(path: string, options?: { forcePrependBaseUrl?: boolean }): string;
}

declare module '@docusaurus/Link' {
  import type {ComponentType} from 'react';
  type LinkLikeProps = {
    to?: string;
    href?: string;
    className?: string;
    children?: React.ReactNode;
    target?: string;
  } & Record<string, unknown>;
  const Link: ComponentType<LinkLikeProps>;
  export default Link;
}

declare module '@docusaurus/useDocusaurusContext' {
  type DocusaurusContext = {
    siteConfig?: {
      title?: string;
      tagline?: string;
      favicon?: string;
    };
  };
  export default function useDocusaurusContext(): {siteConfig: DocusaurusContext['siteConfig']};
}

declare module '@theme/Layout' {
  import type {FC, ReactNode} from 'react';
  export interface Props {
    title?: string;
    description?: string;
    children?: ReactNode;
  }
  const Layout: FC<Props>;
  export default Layout;
}

declare module '@theme/Heading' {
  import type {ComponentType} from 'react';
  const Heading: ComponentType<{
    as?: keyof JSX.IntrinsicElements;
    id?: string;
    className?: string;
    children?: React.ReactNode;
  }>;
  export default Heading;
}

declare module '@theme/*';

export {};
