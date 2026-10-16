import React from 'react';
import ReactDOM from 'react-dom/client';
import { App } from './app/App';
import { ErrorBoundary } from './ui/ErrorBoundary';
import './styles/tokens.css';
import './styles/globals.css';
import './styles/panel.css';
import './styles/dock.css';
import './styles/themes.css';

ReactDOM.createRoot(document.getElementById('root')!).render(
  <React.StrictMode>
    <ErrorBoundary>
      <App />
    </ErrorBoundary>
  </React.StrictMode>,
);
