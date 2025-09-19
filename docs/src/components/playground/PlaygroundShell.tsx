import React from 'react';
import styles from '../PlaygroundLayout.module.css';

type PlaygroundShellProps = {
  themeVars: React.CSSProperties;
  hero: React.ReactNode;
  alerts?: React.ReactNode;
  sidebar: React.ReactNode;
  main: React.ReactNode;
  rightRail?: React.ReactNode;
};

export default function PlaygroundShell({themeVars, hero, alerts, sidebar, main, rightRail}: PlaygroundShellProps): JSX.Element {
  return (
    <div className={styles.breakout}>
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
