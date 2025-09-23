import {useCallback, useRef} from 'react';
import useBaseUrl from '@docusaurus/useBaseUrl';

export function useWorkerMessenger(workerScript = 'wasm/engine/worker.js') {
  const workerRef = useRef<Worker | null>(null);
  const requestId = useRef(0);
  const workerUrl = useBaseUrl(workerScript.startsWith('/') ? workerScript.slice(1) : workerScript);

  const ensureWorker = useCallback(() => {
    if (typeof window === 'undefined') {
      return null;
    }
    if (!workerRef.current) {
      workerRef.current = new Worker(workerUrl, {type: 'module'});
    }
    return workerRef.current;
  }, [workerUrl]);

  const terminateWorker = useCallback(() => {
    if (workerRef.current) {
      workerRef.current.terminate();
      workerRef.current = null;
    }
  }, []);

  const callWorker = useCallback(
    (action: string, payload?: unknown): Promise<any> => {
      const worker = workerRef.current;
      if (!worker) {
        return Promise.reject(new Error('Worker not ready'));
      }
      return new Promise((resolve, reject) => {
        const id = `req_${Date.now()}_${requestId.current++}`;
        const listener = (event: MessageEvent) => {
          const message = event.data as any;
          if (message?.id === id) {
            worker.removeEventListener('message', listener);
            if (message.ok) {
              resolve(message);
            } else {
              reject(new Error(message.error ?? 'Worker error'));
            }
          }
        };
        worker.addEventListener('message', listener);
        worker.postMessage({id, action, payload});
      });
    },
    [],
  );

  return {
    ensureWorker,
    terminateWorker,
    callWorker,
    workerRef,
  } as const;
}
