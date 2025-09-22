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
  boardHexBorder: string;
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
  controlBg: string;
  controlHoverBg: string;
  controlBorder: string;
  controlHoverBorder: string;
  controlFocusShadow: string;
  controlDisabledBg: string;
  controlDisabledBorder: string;
};

export function getPlaygroundPalette(colorMode: 'light' | 'dark'): PlaygroundPalette {
  if (colorMode === 'dark') {
    const teal = '#0f2d2d';
    const tealBorder = 'rgba(60, 120, 120, 0.55)';
    const orange = 'rgba(240, 138, 86, 0.9)';
    return {
      shellBg: `linear-gradient(122deg, ${teal} 0%, rgba(15, 45, 45, 0.92) 100%)`,
      shellBorder: 'rgba(236, 135, 84, 0.35)',
      heroGradient: 'linear-gradient(135deg, rgba(240, 138, 86, 0.28) 0%, rgba(250, 169, 120, 0.22) 100%)',
      heroText: '#e9eef2',
      heroAccent: 'rgba(240, 138, 86, 0.6)',
      heroShadow: '0 40px 95px rgba(8, 20, 20, 0.6)',
      panelBg: 'rgba(12, 28, 28, 0.92)',
      panelBorder: tealBorder,
      panelShadow: '0 30px 60px rgba(8, 20, 20, 0.6)',
      playerActiveBg: 'rgba(240, 138, 86, 0.22)',
      playerBg: 'rgba(10, 24, 24, 0.65)',
      boardCellBg: 'rgba(10, 24, 24, 0.92)',
      boardCellBorder: 'rgba(180, 200, 200, 0.28)',
      boardCellHighlight: 'rgba(240, 138, 86, 0.35)',
      boardHexBorder: tealBorder,
      rackTileBg: 'rgba(16, 34, 34, 0.9)',
      rackTileBorder: 'rgba(180, 200, 200, 0.35)',
      rackTileHighlight: 'rgba(240, 138, 86, 0.28)',
      warningBg: 'rgba(234, 179, 8, 0.15)',
      warningBorder: 'rgba(250, 204, 21, 0.45)',
      errorBg: 'rgba(248, 113, 113, 0.16)',
      errorBorder: 'rgba(248, 113, 113, 0.6)',
      infoBg: 'rgba(240, 138, 86, 0.18)',
      infoBorder: 'rgba(240, 138, 86, 0.5)',
      accentBorder: 'rgba(240, 138, 86, 0.7)',
      textSubtle: 'rgba(225, 235, 240, 0.78)',
      statChipBg: 'rgba(14, 32, 32, 0.8)',
      statChipBorder: tealBorder,
      segmentedBg: 'rgba(12, 28, 28, 0.9)',
      segmentedBorder: tealBorder,
      segmentedActiveBg: 'rgba(240, 138, 86, 0.28)',
      segmentedActiveBorder: 'rgba(240, 138, 86, 0.55)',
      alertShadow: '0 18px 45px rgba(8, 20, 20, 0.55)',
      controlBg: 'rgba(12, 28, 28, 0.88)',
      controlHoverBg: 'rgba(240, 138, 86, 0.22)',
      controlBorder: 'rgba(180, 200, 200, 0.32)',
      controlHoverBorder: 'rgba(240, 138, 86, 0.62)',
      controlFocusShadow: '0 0 0 3px rgba(240, 138, 86, 0.32)',
      controlDisabledBg: 'rgba(16, 34, 34, 0.65)',
      controlDisabledBorder: 'rgba(180, 200, 200, 0.22)',
    };
  }

  return {
    shellBg: 'linear-gradient(128deg, rgba(255, 244, 240, 0.95) 0%, rgba(255, 232, 220, 0.9) 100%)',
    shellBorder: 'rgba(215, 110, 63, 0.25)',
    heroGradient: 'linear-gradient(135deg, rgba(215, 110, 63, 0.22) 0%, rgba(240, 138, 86, 0.18) 100%)',
    heroText: '#161b1d',
    heroAccent: 'rgba(215, 110, 63, 0.5)',
    heroShadow: '0 30px 75px rgba(22, 27, 29, 0.18)',
    panelBg: 'rgba(255, 255, 255, 0.96)',
    panelBorder: 'rgba(215, 110, 63, 0.28)',
    panelShadow: '0 30px 60px rgba(22, 27, 29, 0.12)',
    playerActiveBg: 'rgba(240, 138, 86, 0.25)',
    playerBg: 'rgba(252, 250, 248, 0.95)',
    boardCellBg: '#ffffff',
    boardCellBorder: 'rgba(60, 64, 66, 0.2)',
    boardCellHighlight: 'rgba(215, 110, 63, 0.22)',
    boardHexBorder: 'rgba(60, 64, 66, 0.35)',
    rackTileBg: '#fff8f4',
    rackTileBorder: 'rgba(215, 110, 63, 0.35)',
    rackTileHighlight: 'rgba(215, 110, 63, 0.18)',
    warningBg: 'rgba(251, 191, 36, 0.2)',
    warningBorder: 'rgba(217, 119, 6, 0.4)',
    errorBg: 'rgba(248, 113, 113, 0.18)',
    errorBorder: 'rgba(220, 38, 38, 0.55)',
    infoBg: 'rgba(240, 138, 86, 0.14)',
    infoBorder: 'rgba(215, 110, 63, 0.4)',
    accentBorder: 'rgba(215, 110, 63, 0.55)',
    textSubtle: 'rgba(60, 64, 66, 0.9)',
    statChipBg: 'rgba(253, 241, 235, 0.86)',
    statChipBorder: 'rgba(215, 110, 63, 0.45)',
    segmentedBg: 'rgba(255, 246, 240, 0.94)',
    segmentedBorder: 'rgba(215, 110, 63, 0.35)',
    segmentedActiveBg: 'rgba(215, 110, 63, 0.2)',
    segmentedActiveBorder: 'rgba(240, 138, 86, 0.5)',
    alertShadow: '0 18px 40px rgba(22, 27, 29, 0.16)',
    controlBg: 'rgba(255, 255, 255, 0.96)',
    controlHoverBg: 'rgba(215, 110, 63, 0.14)',
    controlBorder: 'rgba(215, 110, 63, 0.28)',
    controlHoverBorder: 'rgba(215, 110, 63, 0.55)',
    controlFocusShadow: '0 0 0 3px rgba(215, 110, 63, 0.25)',
    controlDisabledBg: 'rgba(255, 248, 244, 0.7)',
    controlDisabledBorder: 'rgba(215, 110, 63, 0.22)',
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
    '--tt-control-bg': palette.controlBg,
    '--tt-control-hover-bg': palette.controlHoverBg,
    '--tt-control-border': palette.controlBorder,
    '--tt-control-hover-border': palette.controlHoverBorder,
    '--tt-control-focus-shadow': palette.controlFocusShadow,
    '--tt-control-disabled-bg': palette.controlDisabledBg,
    '--tt-control-disabled-border': palette.controlDisabledBorder,
    '--tt-hex-border': palette.boardHexBorder,
  } as CSSProperties;
}
