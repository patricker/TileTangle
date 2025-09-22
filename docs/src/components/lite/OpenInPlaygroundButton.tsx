import React, {useMemo} from 'react';

export type OpenInPlaygroundButtonProps = {
  initial: unknown;
  snapshot?: unknown;
  label?: string;
  className?: string;
  path?: string; // default '/docs/playground'
};

function toBase64Url(json: unknown): string {
  const s = JSON.stringify(json);
  if (typeof window === 'undefined') {
    // SSR-safe
    // eslint-disable-next-line no-undef
    // @ts-ignore
    const b64 = Buffer.from(s, 'utf-8').toString('base64');
    return b64.replace(/\+/g, '-').replace(/\//g, '_').replace(/=+$/g, '');
  }
  const b64 = btoa(unescape(encodeURIComponent(s)));
  return b64.replace(/\+/g, '-').replace(/\//g, '_').replace(/=+$/g, '');
}

export default function OpenInPlaygroundButton({initial, snapshot, label = 'Open in Playground', className, path = '/docs/playground'}: OpenInPlaygroundButtonProps): JSX.Element {
  const href = useMemo(() => {
    const params = new URLSearchParams();
    try {
      params.set('initial', toBase64Url(initial));
    } catch {
      // ignore encoding errors, leave param off
    }
    if (snapshot != null) {
      try {
        params.set('snapshot', toBase64Url(snapshot));
      } catch {
        // ignore
      }
    }
    return `${path}?${params.toString()}`;
  }, [initial, snapshot, path]);

  return (
    <a className={className ?? 'button button--primary'} href={href}>
      {label}
    </a>
  );
}

