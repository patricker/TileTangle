import React from 'react';
import styles from '../PlaygroundLayout.module.css';

type PlaygroundShellProps = {
  themeVars: React.CSSProperties;
  hero: React.ReactNode;
  alerts?: React.ReactNode;
  sidebar: React.ReactNode;
  main: React.ReactNode;
  rightRail?: React.ReactNode;
  compact?: boolean;
};

export default function PlaygroundShell({themeVars, hero, alerts, sidebar, main, rightRail, compact}: PlaygroundShellProps): JSX.Element {
  const outerClass = styles.wrap;
  if (compact) {
    return (
      <div className={outerClass}>
        <div className={styles.shell} style={themeVars}>
          {hero}
          {alerts}
          <div className={styles.layoutCompact}>
            <main className={styles.stage}>{main}</main>
            {rightRail && <div className={`${styles.rightRail} ${styles.panelGrid}`}>{rightRail}</div>}
            <div className={`${styles.sidebar} ${styles.panelGrid}`}>{sidebar}</div>
          </div>
        </div>
      </div>
    );
  }
  return (
    <div className={outerClass}>
      <div className={styles.shell} style={themeVars}>
        {hero}
        {alerts}
        <div className={styles.layout}>
          <aside className={styles.sidebar}>{sidebar}</aside>
          <main className={styles.stage}>{main}</main>
          {rightRail && <aside className={styles.rightRail}>{rightRail}</aside>}
        </div>
      </div>
    </div>
  );
}
