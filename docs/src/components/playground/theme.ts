import type {CSSProperties} from 'react';

export type PlaygroundPalette = {
  shellBg: string;
  shellBorder: string;
  heroGradient: string;
  heroText: string;
  heroAccent: string;
  heroShadow: string;
  panelBg: string;
  panelBorder: string;
  panelShadow: string;
  playerActiveBg: string;
  playerBg: string;
  boardCellBg: string;
  boardCellBorder: string;
  boardCellHighlight: string;
  rackTileBg: string;
  rackTileBorder: string;
  rackTileHighlight: string;
  warningBg: string;
  warningBorder: string;
  errorBg: string;
  errorBorder: string;
  infoBg: string;
  infoBorder: string;
  accentBorder: string;
  textSubtle: string;
  statChipBg: string;
  statChipBorder: string;
  segmentedBg: string;
  segmentedBorder: string;
  segmentedActiveBg: string;
  segmentedActiveBorder: string;
  alertShadow: string;
  rackTileShadow?: string;
};

export function getPlaygroundPalette(colorMode: 'light' | 'dark'): PlaygroundPalette {
  if (colorMode === 'dark') {
    return {
      shellBg: 'linear-gradient(122deg, rgba(15, 23, 42, 0.96) 0%, rgba(30, 64, 175, 0.45) 100%)',
      shellBorder: 'rgba(94, 234, 212, 0.22)',
      heroGradient: 'linear-gradient(135deg, rgba(59, 130, 246, 0.32) 0%, rgba(14, 165, 233, 0.28) 55%, rgba(56, 189, 248, 0.22) 100%)',
      heroText: '#e2e8f0',
      heroAccent: 'rgba(125, 211, 252, 0.55)',
      heroShadow: '0 40px 95px rgba(2, 6, 23, 0.6)',
      panelBg: 'rgba(15, 23, 42, 0.82)',
      panelBorder: 'rgba(148, 163, 184, 0.28)',
      panelShadow: '0 30px 60px rgba(2, 6, 23, 0.55)',
      playerActiveBg: 'rgba(56, 189, 248, 0.32)',
      playerBg: 'rgba(17, 24, 39, 0.55)',
      boardCellBg: 'rgba(15, 23, 42, 0.92)',
      boardCellBorder: 'rgba(148, 163, 184, 0.35)',
      boardCellHighlight: 'rgba(125, 211, 252, 0.4)',
      rackTileBg: 'rgba(30, 41, 59, 0.88)',
      rackTileBorder: 'rgba(148, 163, 184, 0.4)',
      rackTileHighlight: 'rgba(56, 189, 248, 0.38)',
      warningBg: 'rgba(234, 179, 8, 0.15)',
      warningBorder: 'rgba(250, 204, 21, 0.45)',
      errorBg: 'rgba(248, 113, 113, 0.16)',
      errorBorder: 'rgba(248, 113, 113, 0.6)',
      infoBg: 'rgba(56, 189, 248, 0.18)',
      infoBorder: 'rgba(129, 199, 212, 0.5)',
      accentBorder: 'rgba(56, 189, 248, 0.68)',
      textSubtle: 'rgba(226, 232, 240, 0.78)',
      statChipBg: 'rgba(30, 41, 59, 0.72)',
      statChipBorder: 'rgba(148, 196, 255, 0.42)',
      segmentedBg: 'rgba(17, 24, 39, 0.78)',
      segmentedBorder: 'rgba(71, 85, 105, 0.65)',
      segmentedActiveBg: 'rgba(56, 189, 248, 0.38)',
      segmentedActiveBorder: 'rgba(125, 211, 252, 0.65)',
      alertShadow: '0 18px 45px rgba(2, 6, 23, 0.55)',
    };
  }

  return {
    shellBg: 'linear-gradient(128deg, rgba(248, 250, 252, 0.95) 0%, rgba(224, 242, 254, 0.9) 100%)',
    shellBorder: 'rgba(148, 163, 184, 0.28)',
    heroGradient: 'linear-gradient(135deg, rgba(59, 130, 246, 0.22) 0%, rgba(14, 165, 233, 0.18) 55%, rgba(2, 132, 199, 0.15) 100%)',
    heroText: '#0f172a',
    heroAccent: 'rgba(59, 130, 246, 0.45)',
    heroShadow: '0 30px 75px rgba(15, 23, 42, 0.25)',
    panelBg: 'rgba(255, 255, 255, 0.96)',
    panelBorder: 'rgba(148, 163, 184, 0.4)',
    panelShadow: '0 30px 60px rgba(15, 23, 42, 0.12)',
    playerActiveBg: 'rgba(147, 197, 253, 0.4)',
    playerBg: 'rgba(241, 245, 249, 0.9)',
    boardCellBg: '#ffffff',
    boardCellBorder: 'rgba(148, 163, 184, 0.38)',
    boardCellHighlight: 'rgba(59, 130, 246, 0.25)',
    rackTileBg: '#f8fafc',
    rackTileBorder: 'rgba(148, 163, 184, 0.45)',
    rackTileHighlight: 'rgba(59, 130, 246, 0.22)',
    warningBg: 'rgba(251, 191, 36, 0.2)',
    warningBorder: 'rgba(217, 119, 6, 0.4)',
    errorBg: 'rgba(248, 113, 113, 0.18)',
    errorBorder: 'rgba(220, 38, 38, 0.55)',
    infoBg: 'rgba(59, 130, 246, 0.14)',
    infoBorder: 'rgba(37, 99, 235, 0.4)',
    accentBorder: 'rgba(59, 130, 246, 0.6)',
    textSubtle: 'rgba(71, 85, 105, 0.9)',
    statChipBg: 'rgba(226, 232, 240, 0.72)',
    statChipBorder: 'rgba(148, 163, 184, 0.55)',
    segmentedBg: 'rgba(244, 247, 255, 0.9)',
    segmentedBorder: 'rgba(148, 163, 184, 0.5)',
    segmentedActiveBg: 'rgba(59, 130, 246, 0.2)',
    segmentedActiveBorder: 'rgba(37, 99, 235, 0.55)',
    alertShadow: '0 18px 40px rgba(15, 23, 42, 0.18)',
  };
}

export function buildThemeVars(palette: PlaygroundPalette): CSSProperties {
  return {
    '--tt-shell-bg': palette.shellBg,
    '--tt-shell-border': palette.shellBorder,
    '--tt-hero-gradient': palette.heroGradient,
    '--tt-hero-text': palette.heroText,
    '--tt-hero-accent': palette.heroAccent,
    '--tt-hero-shadow': palette.heroShadow,
    '--tt-panel-bg': palette.panelBg,
    '--tt-panel-border': palette.panelBorder,
    '--tt-panel-shadow': palette.panelShadow,
    '--tt-accent-border': palette.accentBorder,
    '--tt-text-subtle': palette.textSubtle,
    '--tt-stat-chip-bg': palette.statChipBg,
    '--tt-stat-chip-border': palette.statChipBorder,
    '--tt-segmented-bg': palette.segmentedBg,
    '--tt-segmented-border': palette.segmentedBorder,
    '--tt-segmented-active-bg': palette.segmentedActiveBg,
    '--tt-segmented-active-border': palette.segmentedActiveBorder,
    '--tt-alert-error-bg': palette.errorBg,
    '--tt-alert-error-border': palette.errorBorder,
    '--tt-alert-warning-bg': palette.warningBg,
    '--tt-alert-warning-border': palette.warningBorder,
    '--tt-alert-info-bg': palette.infoBg,
    '--tt-alert-info-border': palette.infoBorder,
    '--tt-alert-shadow': palette.alertShadow,
  } as CSSProperties;
}
