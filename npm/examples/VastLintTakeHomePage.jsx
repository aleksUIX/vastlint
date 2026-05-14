import React, { useEffect, useRef, useState } from 'react';
import { fix, validate } from 'vastlint';

const SAMPLE_BROKEN_XML = [
  '<?xml version="1.0" encoding="UTF-8"?>',
  '\x3cVAST version="4.2"\x3e',
  '  \x3cAd id="1"\x3e',
  '    \x3cInLine\x3e',
  '      \x3cAdSystem\x3eAcme\x3c/AdSystem\x3e',
  '      \x3cAdTitle\x3eBad Values Ad\x3c/AdTitle\x3e',
  '      \x3cAdServingId\x3eSERVING-001\x3c/AdServingId\x3e',
  '      \x3cImpression\x3e\x3c![CDATA[https://track.example.com/impression]]\x3e\x3c/Impression\x3e',
  '      \x3cCreatives\x3e',
  '        \x3cCreative\x3e',
  '          \x3cUniversalAdId idRegistry="ad-id.org"\x3eTEST-001\x3c/UniversalAdId\x3e',
  '          \x3cLinear\x3e',
  '            \x3cDuration\x3e90 seconds\x3c/Duration\x3e',
  '            \x3cMediaFiles\x3e',
  '              \x3cMediaFile delivery="download" type="video/mp4" width="1280" height="720"\x3e',
  '                \x3c![CDATA[https://cdn.example.com/video.mp4]]\x3e',
  '              \x3c/MediaFile\x3e',
  '            \x3c/MediaFiles\x3e',
  '          \x3c/Linear\x3e',
  '        \x3c/Creative\x3e',
  '      \x3c/Creatives\x3e',
  '    \x3c/InLine\x3e',
  '  \x3c/Ad\x3e',
  '\x3c/VAST\x3e',
].join('\n');

const SAMPLE_CLEAN_XML = [
  '<?xml version="1.0" encoding="UTF-8"?>',
  '\x3cVAST version="4.2"\x3e',
  '  \x3cAd id="1"\x3e',
  '    \x3cInLine\x3e',
  '      \x3cAdSystem\x3eAcme\x3c/AdSystem\x3e',
  '      \x3cAdTitle\x3eClean Example Ad\x3c/AdTitle\x3e',
  '      \x3cAdServingId\x3eSERVING-001\x3c/AdServingId\x3e',
  '      \x3cImpression\x3e\x3c![CDATA[https://track.example.com/impression]]\x3e\x3c/Impression\x3e',
  '      \x3cCreatives\x3e',
  '        \x3cCreative\x3e',
  '          \x3cUniversalAdId idRegistry="ad-id.org"\x3eTEST-001\x3c/UniversalAdId\x3e',
  '          \x3cLinear\x3e',
  '            \x3cDuration\x3e00:00:30\x3c/Duration\x3e',
  '            \x3cMediaFiles\x3e',
  '              \x3cMediaFile delivery="progressive" type="video/mp4" width="1280" height="720"\x3e',
  '                \x3c![CDATA[https://cdn.example.com/video.mp4]]\x3e',
  '              \x3c/MediaFile\x3e',
  '            \x3c/MediaFiles\x3e',
  '          \x3c/Linear\x3e',
  '        \x3c/Creative\x3e',
  '      \x3c/Creatives\x3e',
  '    \x3c/InLine\x3e',
  '  \x3c/Ad\x3e',
  '\x3c/VAST\x3e',
].join('\n');

const FILTERS = ['all', 'error', 'warning', 'info'];
const SEVERITY_ORDER = { error: 3, warning: 2, info: 1 };
const SEVERITY_COLORS = {
  error: { border: '#f04438', pill: '#fee4e2', text: '#b42318', line: '#fff1f3' },
  warning: { border: '#f79009', pill: '#fef0c7', text: '#b54708', line: '#fffaeb' },
  info: { border: '#2e90fa', pill: '#d1e9ff', text: '#175cd3', line: '#eff8ff' },
};

function createRuntimeIssue(message) {
  return {
    id: 'APP-runtime-error',
    severity: 'error',
    message,
    path: null,
    spec_ref: 'vastlint runtime',
    line: null,
    col: null,
  };
}

function safeValidate(xml) {
  try {
    return validate(xml);
  } catch (error) {
    return {
      version: null,
      issues: [createRuntimeIssue(error instanceof Error ? error.message : 'Unexpected validator failure')],
      summary: {
        errors: 1,
        warnings: 0,
        infos: 0,
        valid: false,
      },
    };
  }
}

function safeFix(xml) {
  try {
    return fix(xml);
  } catch (error) {
    return {
      xml,
      applied: [],
      remaining: [createRuntimeIssue(error instanceof Error ? error.message : 'Unexpected fix failure')],
    };
  }
}

function highestSeverity(issues) {
  let current = 'info';

  for (const issue of issues) {
    if (SEVERITY_ORDER[issue.severity] > SEVERITY_ORDER[current]) {
      current = issue.severity;
    }
  }

  return current;
}

function formatLocation(issue) {
  if (issue.line == null) {
    return 'Document-level issue';
  }

  return issue.col == null ? `Line ${issue.line}` : `Line ${issue.line}, col ${issue.col}`;
}

function sortIssues(issues) {
  return [...issues].sort((left, right) => {
    const severityDelta = SEVERITY_ORDER[right.severity] - SEVERITY_ORDER[left.severity];

    if (severityDelta !== 0) {
      return severityDelta;
    }

    const lineDelta = (left.line ?? Number.MAX_SAFE_INTEGER) - (right.line ?? Number.MAX_SAFE_INTEGER);

    if (lineDelta !== 0) {
      return lineDelta;
    }

    const colDelta = (left.col ?? 0) - (right.col ?? 0);

    if (colDelta !== 0) {
      return colDelta;
    }

    return left.id.localeCompare(right.id);
  });
}

function groupIssuesByLine(issues) {
  const grouped = {};

  for (const issue of issues) {
    if (issue.line == null) {
      continue;
    }

    if (!grouped[issue.line]) {
      grouped[issue.line] = [];
    }

    grouped[issue.line].push(issue);
  }

  return grouped;
}

function countSeverity(issues, severity) {
  return issues.filter((issue) => issue.severity === severity).length;
}

function severityLabel(value) {
  return value.charAt(0).toUpperCase() + value.slice(1);
}

export function VastLintTakeHomePage() {
  const [xml, setXml] = useState(SAMPLE_BROKEN_XML);
  const [result, setResult] = useState(() => safeValidate(SAMPLE_BROKEN_XML));
  const [filter, setFilter] = useState('all');
  const [selectedLine, setSelectedLine] = useState(null);
  const [fixPreview, setFixPreview] = useState(null);
  const [copiedLabel, setCopiedLabel] = useState('');
  const lineRefs = useRef({});
  const copyResetRef = useRef(null);

  const fixSourceXml = fixPreview ? fixPreview.sourceXml : null;

  useEffect(() => {
    const timeoutId = setTimeout(() => {
      setResult(safeValidate(xml));

      if (fixSourceXml !== null && fixSourceXml !== xml) {
        setFixPreview(null);
      }
    }, 120);

    return () => clearTimeout(timeoutId);
  }, [xml, fixSourceXml]);

  useEffect(() => {
    return () => {
      if (copyResetRef.current) {
        clearTimeout(copyResetRef.current);
      }
    };
  }, []);

  const visibleIssues = sortIssues(
    filter === 'all' ? result.issues : result.issues.filter((issue) => issue.severity === filter)
  );
  const issuesByLine = groupIssuesByLine(visibleIssues);
  const xmlLines = xml.replace(/\r\n/g, '\n').split('\n');
  const lineNumberWidth = String(xmlLines.length).length;

  async function handleCopy(value, label) {
    if (typeof navigator === 'undefined' || !navigator.clipboard) {
      return;
    }

    try {
      await navigator.clipboard.writeText(value);
      setCopiedLabel(label);
      if (copyResetRef.current) {
        clearTimeout(copyResetRef.current);
      }
      copyResetRef.current = setTimeout(() => setCopiedLabel(''), 1500);
    } catch {
      setCopiedLabel('Copy failed');
      if (copyResetRef.current) {
        clearTimeout(copyResetRef.current);
      }
      copyResetRef.current = setTimeout(() => setCopiedLabel(''), 1500);
    }
  }

  function jumpToIssue(issue) {
    if (issue.line == null) {
      setSelectedLine(null);
      return;
    }

    setSelectedLine(issue.line);
    const target = lineRefs.current[issue.line];

    if (target) {
      target.scrollIntoView({ behavior: 'smooth', block: 'center' });
    }
  }

  function runAutoFix() {
    setFixPreview({ sourceXml: xml, result: safeFix(xml) });
  }

  function applyFixPreview() {
    if (!fixPreview) {
      return;
    }

    setXml(fixPreview.result.xml);
    setSelectedLine(null);
    setFixPreview(null);
  }

  return (
    <div style={styles.page}>
      <div style={styles.shell}>
        <header style={styles.hero}>
          <div style={styles.heroCopy}>
            <div style={styles.kicker}>React drop-in example</div>
            <h1 style={styles.title}>VAST QA page you can paste into any React app</h1>
            <p style={styles.subtitle}>
              This component uses only <code style={styles.inlineCode}>react</code> and{' '}
              <code style={styles.inlineCode}>vastlint</code>. It gives product teams a ready-made
              VAST validation page with live linting, line-aware issue navigation, and auto-fix
              preview.
            </p>
          </div>

          <div style={styles.heroActions}>
            <button type="button" style={styles.primaryButton} onClick={() => setXml(SAMPLE_BROKEN_XML)}>
              Load broken sample
            </button>
            <button type="button" style={styles.secondaryButton} onClick={() => setXml(SAMPLE_CLEAN_XML)}>
              Load clean sample
            </button>
            <button type="button" style={styles.secondaryButton} onClick={runAutoFix}>
              Auto-fix preview
            </button>
            <button type="button" style={styles.secondaryButton} onClick={() => handleCopy(xml, 'Current XML copied')}>
              Copy current XML
            </button>
            <div style={styles.copyStatus}>{copiedLabel || 'Validation runs as you type.'}</div>
          </div>
        </header>

        <section style={styles.summaryGrid}>
          <div style={styles.summaryCard}>
            <div style={styles.summaryLabel}>Detected version</div>
            <div style={styles.summaryValue}>{result.version ?? 'Unknown'}</div>
          </div>
          <div style={styles.summaryCard}>
            <div style={styles.summaryLabel}>Errors</div>
            <div style={{ ...styles.summaryValue, color: SEVERITY_COLORS.error.text }}>
              {result.summary.errors}
            </div>
          </div>
          <div style={styles.summaryCard}>
            <div style={styles.summaryLabel}>Warnings</div>
            <div style={{ ...styles.summaryValue, color: SEVERITY_COLORS.warning.text }}>
              {result.summary.warnings}
            </div>
          </div>
          <div style={styles.summaryCard}>
            <div style={styles.summaryLabel}>Infos</div>
            <div style={{ ...styles.summaryValue, color: SEVERITY_COLORS.info.text }}>
              {result.summary.infos}
            </div>
          </div>
          <div style={styles.summaryCard}>
            <div style={styles.summaryLabel}>Status</div>
            <div style={{ ...styles.summaryValue, color: result.summary.valid ? '#027a48' : SEVERITY_COLORS.error.text }}>
              {result.summary.valid ? 'Passing' : 'Needs attention'}
            </div>
          </div>
        </section>

        {fixPreview ? (
          <section style={styles.fixPanel}>
            <div style={styles.panelHeader}>
              <div>
                <h2 style={styles.panelTitle}>Auto-fix preview</h2>
                <p style={styles.panelHint}>
                  {fixPreview.result.applied.length > 0
                    ? `${fixPreview.result.applied.length} change(s) can be applied automatically.`
                    : 'No automatic fixes were available for the current XML.'}
                </p>
              </div>

              <div style={styles.fixActions}>
                <button type="button" style={styles.secondaryButton} onClick={() => handleCopy(fixPreview.result.xml, 'Fixed XML copied')}>
                  Copy fixed XML
                </button>
                <button
                  type="button"
                  style={styles.primaryButton}
                  onClick={applyFixPreview}
                  disabled={fixPreview.result.applied.length === 0}
                >
                  Replace editor with fixed XML
                </button>
              </div>
            </div>

            <div style={styles.fixGrid}>
              <div style={styles.fixList}>
                <div style={styles.subsectionTitle}>Applied fixes</div>
                {fixPreview.result.applied.length === 0 ? (
                  <div style={styles.emptyState}>Nothing to apply automatically.</div>
                ) : (
                  fixPreview.result.applied.map((applied, index) => (
                    <div key={`${applied.rule_id}-${index}`} style={styles.fixItem}>
                      <div style={styles.fixRule}>{applied.rule_id}</div>
                      <div style={styles.fixDescription}>{applied.description}</div>
                      <div style={styles.fixPath}>{applied.path}</div>
                    </div>
                  ))
                )}
              </div>

              <div style={styles.fixList}>
                <div style={styles.subsectionTitle}>Remaining issues</div>
                {fixPreview.result.remaining.length === 0 ? (
                  <div style={styles.emptyState}>No remaining issues after auto-fix.</div>
                ) : (
                  sortIssues(fixPreview.result.remaining).map((issue, index) => (
                    <div key={`${issue.id}-${index}`} style={styles.fixItem}>
                      <div style={styles.issueMetaRow}>
                        <span style={{ ...styles.pill, ...pillStyle(issue.severity) }}>
                          {severityLabel(issue.severity)}
                        </span>
                        <span style={styles.fixRule}>{issue.id}</span>
                      </div>
                      <div style={styles.fixDescription}>{issue.message}</div>
                      <div style={styles.fixPath}>{formatLocation(issue)}</div>
                    </div>
                  ))
                )}
              </div>
            </div>

            <pre style={styles.fixPreviewCode}>{fixPreview.result.xml}</pre>
          </section>
        ) : null}

        <div style={styles.topRow}>
          <section style={styles.panel}>
            <div style={styles.panelHeader}>
              <div>
                <h2 style={styles.panelTitle}>Editable VAST input</h2>
                <p style={styles.panelHint}>Paste a tag, wire this to your own fetch flow, or seed it from a bid response.</p>
              </div>
            </div>

            <textarea
              value={xml}
              onChange={(event) => setXml(event.target.value)}
              spellCheck={false}
              style={styles.textarea}
            />
          </section>

          <section style={styles.panelNarrow}>
            <div style={styles.panelHeader}>
              <div>
                <h2 style={styles.panelTitle}>Issues</h2>
                <p style={styles.panelHint}>Filter by severity, then click any issue to jump to the source line.</p>
              </div>
            </div>

            <div style={styles.filterRow}>
              {FILTERS.map((value) => (
                <button
                  key={value}
                  type="button"
                  style={{
                    ...styles.filterButton,
                    ...(filter === value ? styles.filterButtonActive : null),
                  }}
                  onClick={() => setFilter(value)}
                >
                  {value === 'all' ? 'All' : severityLabel(value)}
                  <span style={styles.filterCount}>
                    {value === 'all' ? result.issues.length : countSeverity(result.issues, value)}
                  </span>
                </button>
              ))}
            </div>

            {visibleIssues.length === 0 ? (
              <div style={styles.emptyState}>
                {result.summary.valid
                  ? 'No issues in the current filter. This tag passes the validator.'
                  : 'No issues match the current filter.'}
              </div>
            ) : (
              <div style={styles.issueList}>
                {visibleIssues.map((issue, index) => (
                  <button
                    key={`${issue.id}-${index}`}
                    type="button"
                    style={{
                      ...styles.issueCard,
                      borderLeftColor: SEVERITY_COLORS[issue.severity].border,
                    }}
                    onClick={() => jumpToIssue(issue)}
                  >
                    <div style={styles.issueMetaRow}>
                      <span style={{ ...styles.pill, ...pillStyle(issue.severity) }}>
                        {severityLabel(issue.severity)}
                      </span>
                      <span style={styles.issueId}>{issue.id}</span>
                    </div>

                    <div style={styles.issueMessage}>{issue.message}</div>

                    <div style={styles.issueDetails}>{formatLocation(issue)}</div>
                    <div style={styles.issueDetails}>{issue.path ?? 'No XPath location'}</div>
                    <div style={styles.issueDetails}>{issue.spec_ref}</div>
                  </button>
                ))}
              </div>
            )}
          </section>
        </div>

        <section style={styles.panel}>
          <div style={styles.panelHeader}>
            <div>
              <h2 style={styles.panelTitle}>Line-aware source viewer</h2>
              <p style={styles.panelHint}>Highlighted lines reflect the current issue filter. Click a line number to inspect it.</p>
            </div>
            <div style={styles.viewerMeta}>
              {selectedLine == null ? 'No line selected' : `Selected line ${selectedLine}`}
            </div>
          </div>

          <div style={styles.viewer}>
            {xmlLines.map((lineText, index) => {
              const lineNumber = index + 1;
              const lineIssues = issuesByLine[lineNumber] || [];
              const lineSeverity = lineIssues.length > 0 ? highestSeverity(lineIssues) : null;

              return (
                <div
                  key={lineNumber}
                  ref={(node) => {
                    lineRefs.current[lineNumber] = node;
                  }}
                  style={{
                    ...styles.viewerRow,
                    ...(selectedLine === lineNumber ? styles.viewerRowSelected : null),
                    ...(lineSeverity ? { backgroundColor: SEVERITY_COLORS[lineSeverity].line } : null),
                  }}
                >
                  <button type="button" style={styles.lineNumber} onClick={() => setSelectedLine(lineNumber)}>
                    {String(lineNumber).padStart(lineNumberWidth, '0')}
                  </button>
                  <div style={styles.codeLine}>{lineText || ' '}</div>
                  {lineIssues.length > 0 ? (
                    <span style={{ ...styles.lineIssueCount, color: SEVERITY_COLORS[lineSeverity].text }}>
                      {lineIssues.length}
                    </span>
                  ) : null}
                </div>
              );
            })}
          </div>

          <div style={styles.lineDetailBox}>
            {selectedLine == null || !issuesByLine[selectedLine] ? (
              <div style={styles.panelHint}>Select a highlighted line to inspect its issues here.</div>
            ) : (
              issuesByLine[selectedLine].map((issue, index) => (
                <div key={`${issue.id}-${index}`} style={styles.lineDetailItem}>
                  <div style={styles.issueMetaRow}>
                    <span style={{ ...styles.pill, ...pillStyle(issue.severity) }}>
                      {severityLabel(issue.severity)}
                    </span>
                    <span style={styles.issueId}>{issue.id}</span>
                  </div>
                  <div style={styles.issueMessage}>{issue.message}</div>
                  <div style={styles.issueDetails}>{issue.path ?? 'No XPath location'}</div>
                  <div style={styles.issueDetails}>{issue.spec_ref}</div>
                </div>
              ))
            )}
          </div>
        </section>
      </div>
    </div>
  );
}

export default VastLintTakeHomePage;

function pillStyle(severity) {
  return {
    backgroundColor: SEVERITY_COLORS[severity].pill,
    color: SEVERITY_COLORS[severity].text,
    borderColor: SEVERITY_COLORS[severity].border,
  };
}

const styles = {
  page: {
    minHeight: '100vh',
    background:
      'radial-gradient(circle at top left, rgba(203, 213, 225, 0.45), transparent 30%), linear-gradient(180deg, #f8fafc 0%, #eef2ff 100%)',
    color: '#0f172a',
    fontFamily: 'ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif',
  },
  shell: {
    maxWidth: '1440px',
    margin: '0 auto',
    padding: '32px 20px 48px',
  },
  hero: {
    display: 'flex',
    flexWrap: 'wrap',
    gap: '24px',
    justifyContent: 'space-between',
    alignItems: 'flex-start',
    padding: '24px',
    borderRadius: '24px',
    background: 'rgba(255, 255, 255, 0.8)',
    backdropFilter: 'blur(16px)',
    border: '1px solid rgba(148, 163, 184, 0.25)',
    boxShadow: '0 24px 80px rgba(15, 23, 42, 0.08)',
  },
  heroCopy: {
    flex: '1 1 560px',
  },
  kicker: {
    display: 'inline-flex',
    padding: '6px 10px',
    borderRadius: '999px',
    backgroundColor: '#dbeafe',
    color: '#1d4ed8',
    fontSize: '12px',
    fontWeight: 700,
    textTransform: 'uppercase',
    letterSpacing: '0.08em',
  },
  title: {
    margin: '16px 0 12px',
    fontSize: 'clamp(2rem, 4vw, 3.5rem)',
    lineHeight: 1,
    letterSpacing: '-0.04em',
  },
  subtitle: {
    margin: 0,
    maxWidth: '70ch',
    fontSize: '16px',
    lineHeight: 1.7,
    color: '#334155',
  },
  inlineCode: {
    fontFamily: 'ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace',
    fontSize: '0.95em',
    backgroundColor: '#e2e8f0',
    padding: '1px 6px',
    borderRadius: '6px',
  },
  heroActions: {
    flex: '0 1 360px',
    display: 'grid',
    gap: '10px',
  },
  primaryButton: {
    appearance: 'none',
    border: 'none',
    borderRadius: '14px',
    padding: '14px 16px',
    backgroundColor: '#0f172a',
    color: '#f8fafc',
    fontSize: '14px',
    fontWeight: 700,
    cursor: 'pointer',
  },
  secondaryButton: {
    appearance: 'none',
    border: '1px solid #cbd5e1',
    borderRadius: '14px',
    padding: '14px 16px',
    backgroundColor: '#ffffff',
    color: '#0f172a',
    fontSize: '14px',
    fontWeight: 700,
    cursor: 'pointer',
  },
  copyStatus: {
    minHeight: '22px',
    fontSize: '13px',
    color: '#475569',
  },
  summaryGrid: {
    display: 'grid',
    gap: '14px',
    gridTemplateColumns: 'repeat(auto-fit, minmax(160px, 1fr))',
    marginTop: '18px',
  },
  summaryCard: {
    padding: '18px',
    borderRadius: '18px',
    backgroundColor: 'rgba(255, 255, 255, 0.85)',
    border: '1px solid rgba(148, 163, 184, 0.2)',
    boxShadow: '0 8px 30px rgba(15, 23, 42, 0.05)',
  },
  summaryLabel: {
    fontSize: '12px',
    fontWeight: 700,
    letterSpacing: '0.08em',
    textTransform: 'uppercase',
    color: '#64748b',
  },
  summaryValue: {
    marginTop: '8px',
    fontSize: '30px',
    lineHeight: 1,
    fontWeight: 800,
  },
  fixPanel: {
    marginTop: '18px',
    padding: '20px',
    borderRadius: '24px',
    backgroundColor: '#fffdf8',
    border: '1px solid #fde68a',
    boxShadow: '0 16px 40px rgba(180, 83, 9, 0.08)',
  },
  panel: {
    marginTop: '18px',
    padding: '20px',
    borderRadius: '24px',
    backgroundColor: 'rgba(255, 255, 255, 0.86)',
    border: '1px solid rgba(148, 163, 184, 0.2)',
    boxShadow: '0 16px 40px rgba(15, 23, 42, 0.05)',
  },
  panelNarrow: {
    marginTop: '18px',
    padding: '20px',
    borderRadius: '24px',
    backgroundColor: 'rgba(255, 255, 255, 0.86)',
    border: '1px solid rgba(148, 163, 184, 0.2)',
    boxShadow: '0 16px 40px rgba(15, 23, 42, 0.05)',
    flex: '0 1 430px',
    minWidth: '320px',
  },
  panelHeader: {
    display: 'flex',
    flexWrap: 'wrap',
    justifyContent: 'space-between',
    gap: '12px',
    alignItems: 'flex-start',
    marginBottom: '16px',
  },
  panelTitle: {
    margin: 0,
    fontSize: '20px',
    lineHeight: 1.2,
  },
  panelHint: {
    margin: '6px 0 0',
    fontSize: '14px',
    lineHeight: 1.6,
    color: '#475569',
  },
  topRow: {
    display: 'flex',
    flexWrap: 'wrap',
    gap: '18px',
    alignItems: 'stretch',
  },
  textarea: {
    width: '100%',
    minHeight: '480px',
    resize: 'vertical',
    borderRadius: '18px',
    border: '1px solid #cbd5e1',
    backgroundColor: '#0f172a',
    color: '#e2e8f0',
    padding: '18px',
    fontSize: '13px',
    lineHeight: 1.7,
    fontFamily: 'ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace',
  },
  filterRow: {
    display: 'flex',
    flexWrap: 'wrap',
    gap: '10px',
    marginBottom: '14px',
  },
  filterButton: {
    appearance: 'none',
    border: '1px solid #cbd5e1',
    borderRadius: '999px',
    backgroundColor: '#fff',
    color: '#334155',
    padding: '10px 14px',
    fontSize: '13px',
    fontWeight: 700,
    cursor: 'pointer',
    display: 'inline-flex',
    gap: '8px',
    alignItems: 'center',
  },
  filterButtonActive: {
    backgroundColor: '#e0e7ff',
    borderColor: '#a5b4fc',
    color: '#3730a3',
  },
  filterCount: {
    minWidth: '18px',
    textAlign: 'center',
  },
  issueList: {
    display: 'grid',
    gap: '12px',
    maxHeight: '520px',
    overflowY: 'auto',
    paddingRight: '4px',
  },
  issueCard: {
    textAlign: 'left',
    padding: '14px',
    backgroundColor: '#ffffff',
    border: '1px solid #e2e8f0',
    borderLeftWidth: '6px',
    borderRadius: '16px',
    cursor: 'pointer',
  },
  issueMetaRow: {
    display: 'flex',
    flexWrap: 'wrap',
    gap: '8px',
    alignItems: 'center',
  },
  pill: {
    display: 'inline-flex',
    alignItems: 'center',
    border: '1px solid transparent',
    borderRadius: '999px',
    padding: '3px 8px',
    fontSize: '11px',
    fontWeight: 800,
    letterSpacing: '0.04em',
    textTransform: 'uppercase',
  },
  issueId: {
    fontFamily: 'ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace',
    fontSize: '12px',
    color: '#334155',
  },
  issueMessage: {
    marginTop: '10px',
    fontSize: '14px',
    lineHeight: 1.6,
    color: '#0f172a',
  },
  issueDetails: {
    marginTop: '6px',
    fontSize: '12px',
    color: '#64748b',
    wordBreak: 'break-word',
    fontFamily: 'ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace',
  },
  viewerMeta: {
    fontSize: '13px',
    color: '#475569',
  },
  viewer: {
    borderRadius: '18px',
    border: '1px solid #dbeafe',
    backgroundColor: '#f8fafc',
    overflow: 'auto',
    maxHeight: '520px',
  },
  viewerRow: {
    display: 'grid',
    gridTemplateColumns: '72px 1fr 40px',
    gap: '12px',
    alignItems: 'flex-start',
    padding: '8px 12px',
    borderBottom: '1px solid rgba(148, 163, 184, 0.12)',
  },
  viewerRowSelected: {
    boxShadow: 'inset 0 0 0 2px #0f172a',
  },
  lineNumber: {
    appearance: 'none',
    border: 'none',
    background: 'transparent',
    padding: 0,
    margin: 0,
    color: '#64748b',
    fontFamily: 'ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace',
    fontSize: '12px',
    textAlign: 'right',
    cursor: 'pointer',
  },
  codeLine: {
    minHeight: '20px',
    whiteSpace: 'pre-wrap',
    wordBreak: 'break-word',
    fontFamily: 'ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace',
    fontSize: '13px',
    lineHeight: 1.6,
    color: '#0f172a',
  },
  lineIssueCount: {
    alignSelf: 'center',
    justifySelf: 'end',
    fontSize: '12px',
    fontWeight: 800,
  },
  lineDetailBox: {
    marginTop: '14px',
    display: 'grid',
    gap: '10px',
  },
  lineDetailItem: {
    padding: '14px',
    borderRadius: '16px',
    backgroundColor: '#ffffff',
    border: '1px solid #e2e8f0',
  },
  emptyState: {
    padding: '18px',
    borderRadius: '16px',
    backgroundColor: '#f8fafc',
    color: '#475569',
    fontSize: '14px',
    lineHeight: 1.6,
  },
  fixActions: {
    display: 'flex',
    flexWrap: 'wrap',
    gap: '10px',
  },
  fixGrid: {
    display: 'grid',
    gridTemplateColumns: 'repeat(auto-fit, minmax(260px, 1fr))',
    gap: '16px',
    marginBottom: '16px',
  },
  fixList: {
    display: 'grid',
    gap: '10px',
  },
  subsectionTitle: {
    fontSize: '13px',
    fontWeight: 800,
    letterSpacing: '0.06em',
    textTransform: 'uppercase',
    color: '#92400e',
  },
  fixItem: {
    padding: '14px',
    borderRadius: '16px',
    backgroundColor: '#ffffff',
    border: '1px solid #fcd34d',
  },
  fixRule: {
    fontFamily: 'ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace',
    fontSize: '12px',
    color: '#78350f',
  },
  fixDescription: {
    marginTop: '8px',
    fontSize: '14px',
    lineHeight: 1.6,
    color: '#451a03',
  },
  fixPath: {
    marginTop: '8px',
    fontSize: '12px',
    lineHeight: 1.5,
    color: '#92400e',
    wordBreak: 'break-word',
    fontFamily: 'ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace',
  },
  fixPreviewCode: {
    margin: 0,
    padding: '16px',
    borderRadius: '18px',
    border: '1px solid #fcd34d',
    backgroundColor: '#fffdf8',
    maxHeight: '260px',
    overflow: 'auto',
    whiteSpace: 'pre-wrap',
    wordBreak: 'break-word',
    fontFamily: 'ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace',
    fontSize: '13px',
    lineHeight: 1.6,
  },
};