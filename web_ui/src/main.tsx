import React from 'react';
import ReactDOM from 'react-dom/client';
import App from './App';
import { registerPealayerServiceWorker } from './webPlatform';

ReactDOM.createRoot(document.getElementById('root')!).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>
);

window.addEventListener('load', () => {
  void registerPealayerServiceWorker().catch((error) => {
    console.warn('Pealayer offline support could not start', error);
  });
});
