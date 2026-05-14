# React Drop-in Example

This folder contains a single-file React page that can be copied into an existing app with minimal changes.

## Files

- `VastLintTakeHomePage.jsx` - full page component with live validation, issue filtering, line-aware source navigation, and auto-fix preview.

## Requirements

Install the same runtime dependencies the example imports:

```sh
npm install react vastlint
```

Use a bundler environment already supported by the `vastlint` package, such as Vite, Webpack 5, or Rollup.

## Usage

Copy `VastLintTakeHomePage.jsx` into your app, then render it from a route or top-level page:

```jsx
import VastLintTakeHomePage from './VastLintTakeHomePage';

export default function App() {
  return <VastLintTakeHomePage />;
}
```

## What The Page Includes

- Live VAST validation as the user types or pastes XML
- Summary cards for version and severity counts
- Filterable issue list with click-to-jump line navigation
- Read-only source viewer with highlighted problem lines
- Auto-fix preview with replace-in-editor and copy actions

The component uses inline styles so it can be dropped into a project without CSS setup. Replace those styles with your design system primitives when integrating into a production UI.