import React from 'react';
import styles from '../PlaygroundLayout.module.css';

type AlertKind = 'error' | 'warning' | 'info';

export type AlertItem = {
  id?: string;
  kind: AlertKind;
  message: React.ReactNode;
};

export type AlertStackProps = {
  alerts: AlertItem[];
};

const kindToClass: Record<AlertKind, string> = {
  error: `${styles.alert} ${styles.alertError}`,
  warning: `${styles.alert} ${styles.alertWarning}`,
  info: `${styles.alert} ${styles.alertInfo}`,
};

const AlertStack: React.FC<AlertStackProps> = ({alerts}) => {
  const visible = alerts.filter(alert => alert && alert.message != null && alert.message !== '');
  if (visible.length === 0) {
    return null;
  }
  return (
    <div className={styles.alertStack}>
      {visible.map((alert, index) => (
        <div key={alert.id ?? index} className={kindToClass[alert.kind]}>
          {alert.message}
        </div>
      ))}
    </div>
  );
};

export default AlertStack;
