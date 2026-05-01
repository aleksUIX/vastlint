#!/usr/bin/env bash
# Smoke tests for --vast-version and --ignore-pattern CLI flags.
# Run from the vastlint/ directory: bash scripts/smoke-test-new-flags.sh
set -euo pipefail

BIN="${1:-./target/debug/vastlint}"
PASS=0; FAIL=0

ok()  { echo "PASS [$1]"; PASS=$((PASS+1)); }
fail(){ echo "FAIL [$1]: $2"; FAIL=$((FAIL+1)); }
jv()  { python3 -c "import json; d=json.load(open('$1')); print(d$2)" 2>/dev/null; }
jhas(){ python3 -c "import json; d=json.load(open('$1')); print(any(i['id']==\"$2\" for i in d['issues']))" 2>/dev/null; }

# ─── Fixtures written to tmp files to avoid quoting nightmares ───────────────
cat > /tmp/v2_inline.xml <<'EOF'
<VAST version="2.0">
  <Ad><InLine>
    <AdSystem>Test</AdSystem>
    <AdTitle>Test Ad</AdTitle>
    <Impression><![CDATA[https://t.example.com/imp]]></Impression>
    <Creatives><Creative><Linear>
      <Duration>00:00:30</Duration>
      <MediaFiles>
        <MediaFile type="video/mp4" width="640" height="480" bitrate="500" delivery="progressive">
          <![CDATA[https://cdn.example.com/ad.mp4]]>
        </MediaFile>
      </MediaFiles>
    </Linear></Creative></Creatives>
  </InLine></Ad>
</VAST>
EOF

cat > /tmp/v41_inline.xml <<'EOF'
<VAST version="4.1">
  <Ad><InLine>
    <AdSystem>Test</AdSystem>
    <AdServingId>srv-abc</AdServingId>
    <AdTitle>Test Ad</AdTitle>
    <Impression><![CDATA[https://t.example.com/imp]]></Impression>
    <Creatives><Creative>
      <UniversalAdId idRegistry="Ad-ID">abc123</UniversalAdId>
      <Linear>
        <Duration>00:00:30</Duration>
        <TrackingEvents>
          <Tracking event="start"><![CDATA[https://t.example.com/start]]></Tracking>
          <Tracking event="firstQuartile"><![CDATA[https://t.example.com/q1]]></Tracking>
          <Tracking event="midpoint"><![CDATA[https://t.example.com/mid]]></Tracking>
          <Tracking event="thirdQuartile"><![CDATA[https://t.example.com/q3]]></Tracking>
          <Tracking event="complete"><![CDATA[https://t.example.com/complete]]></Tracking>
        </TrackingEvents>
        <MediaFiles>
          <MediaFile type="video/mp4" width="640" height="480" bitrate="500" delivery="progressive">
            <![CDATA[https://cdn.example.com/ad.mp4]]>
          </MediaFile>
        </MediaFiles>
      </Linear>
    </Creative></Creatives>
  </InLine></Ad>
</VAST>
EOF

cat > /tmp/vmacro.xml <<'EOF'
<VAST version="4.2">
  <Ad><InLine>
    <AdSystem>Acme DSP</AdSystem>
    <AdServingId>srv-macro-01</AdServingId>
    <AdTitle>Macro Test Ad</AdTitle>
    <Impression><![CDATA[${IMP_URL}]]></Impression>
    <Creatives><Creative>
      <UniversalAdId idRegistry="Ad-ID">mac01</UniversalAdId>
      <Linear>
        <Duration>00:00:30</Duration>
        <TrackingEvents>
          <Tracking event="start"><![CDATA[%%TRACKER_START%%]]></Tracking>
          <Tracking event="firstQuartile"><![CDATA[${Q1_URL}]]></Tracking>
          <Tracking event="midpoint"><![CDATA[${MID_URL}]]></Tracking>
          <Tracking event="thirdQuartile"><![CDATA[${Q3_URL}]]></Tracking>
          <Tracking event="complete"><![CDATA[${COMP_URL}]]></Tracking>
        </TrackingEvents>
        <MediaFiles>
          <MediaFile type="video/mp4" width="1920" height="1080" bitrate="2000" delivery="progressive">
            <![CDATA[https://cdn.example.com/hd.mp4]]>
          </MediaFile>
        </MediaFiles>
      </Linear>
    </Creative></Creatives>
  </InLine></Ad>
</VAST>
EOF

echo "────────────────────────────────────────────────────────────"
echo " vastlint --vast-version / --ignore-pattern smoke tests"
echo "────────────────────────────────────────────────────────────"

# ─── 1. --vast-version 2.0→4.1: result version field is "4.1" ────────────────
$BIN check /tmp/v2_inline.xml --vast-version 4.1 --format json > /tmp/r1.json 2>/dev/null || true
v=$(jv /tmp/r1.json "['version']")
[[ "$v" == "4.1" ]] && ok "1: force-2.0-to-4.1 version field == 4.1" \
                      || fail "1: force-2.0-to-4.1 version field" "got '$v'"

# ─── 2. --vast-version 2.0→4.1: VAST-4.1-adservingid-present fires ───────────
v=$(jhas /tmp/r1.json "VAST-4.1-adservingid-present")
[[ "$v" == "True" ]] && ok "2: force-4.1 fires adservingid rule" \
                       || fail "2: force-4.1 fires adservingid rule" "got '$v'"

# ─── 3. --vast-version 4.1→2.0: adservingid rule absent ─────────────────────
$BIN check /tmp/v41_inline.xml --vast-version 2.0 --format json > /tmp/r3.json 2>/dev/null || true
v=$(jhas /tmp/r3.json "VAST-4.1-adservingid-present")
[[ "$v" == "False" ]] && ok "3: force-2.0 suppresses adservingid rule" \
                        || fail "3: force-2.0 suppresses adservingid rule" "got '$v'"

# ─── 4. --vast-version 4.1→2.0: version field is "2.0" ──────────────────────
v=$(jv /tmp/r3.json "['version']")
[[ "$v" == "2.0" ]] && ok "4: force-2.0 version field == 2.0" \
                      || fail "4: force-2.0 version field" "got '$v'"

# ─── 5. --vast-version same as declared: no regression ───────────────────────
$BIN check /tmp/v41_inline.xml --vast-version 4.1 --format json > /tmp/r5.json 2>/dev/null || true
v=$(jv /tmp/r5.json "['version']")
[[ "$v" == "4.1" ]] && ok "5: same version no regression" \
                      || fail "5: same version no regression" "got '$v'"

# ─── 6. --vast-version invalid value exits non-zero ──────────────────────────
$BIN check /tmp/v2_inline.xml --vast-version 9.9 >/dev/null 2>&1 && ec=0 || ec=$?
[[ $ec -ne 0 ]] && ok "6: invalid --vast-version exits non-zero" \
                  || fail "6: invalid --vast-version exits non-zero" "exit was 0"

# ─── 7. --ignore-pattern: \${MACRO} style → URL rules don't fire ─────────────
$BIN check /tmp/vmacro.xml --ignore-pattern '\$\{[^}]+\}|%%[^%]+%%' --format json > /tmp/r7.json 2>/dev/null || true
v=$(jhas /tmp/r7.json "VAST-2.0-url-empty")
[[ "$v" == "False" ]] && ok "7: dollar-brace macros replaced, no url-empty error" \
                        || fail "7: dollar-brace macros replaced, no url-empty error" "got '$v'"

# ─── 8. --ignore-pattern: %%MACRO%% style covered by same regex ──────────────
# If %%TRACKER_START%% were left in, url-invalid would fire. Verify it doesn't.
$BIN check /tmp/vmacro.xml --ignore-pattern '\$\{[^}]+\}|%%[^%]+%%' --format json > /tmp/r8.json 2>/dev/null || true
issues=$(python3 -c "import json; d=json.load(open('/tmp/r8.json')); print([i['id'] for i in d['issues'] if 'url' in i['id'].lower()])" 2>/dev/null)
[[ "$issues" == "[]" ]] && ok "8: percent-percent macros replaced, no URL issues" \
                           || fail "8: percent-percent macros replaced, no URL issues" "url issues: $issues"

# ─── 9. --ignore-pattern invalid regex exits non-zero ────────────────────────
$BIN check /tmp/v2_inline.xml --ignore-pattern '[invalid' >/dev/null 2>&1 && ec=0 || ec=$?
[[ $ec -ne 0 ]] && ok "9: invalid --ignore-pattern exits non-zero" \
                  || fail "9: invalid --ignore-pattern exits non-zero" "exit was 0"

# ─── 10. --ignore-pattern on fix subcommand works ────────────────────────────
$BIN fix /tmp/v41_inline.xml --ignore-pattern '\$\{[^}]+\}' > /dev/null 2>&1 && ec=0 || ec=$?
[[ $ec -eq 0 ]] && ok "10: fix --ignore-pattern exits zero" \
                  || fail "10: fix --ignore-pattern exits zero" "exit was $ec"

# ─── 11. --vast-version on fix subcommand works ──────────────────────────────
$BIN fix /tmp/v41_inline.xml --vast-version 4.1 > /dev/null 2>&1 && ec=0 || ec=$?
[[ $ec -eq 0 ]] && ok "11: fix --vast-version exits zero" \
                  || fail "11: fix --vast-version exits zero" "exit was $ec"

# ─── 12. both flags together: correct version and no macro URL errors ─────────
$BIN check /tmp/vmacro.xml --vast-version 4.2 --ignore-pattern '\$\{[^}]+\}|%%[^%]+%%' --format json > /tmp/r12.json 2>/dev/null || true
v=$(jv /tmp/r12.json "['version']")
[[ "$v" == "4.2" ]] && ok "12: both flags — version is 4.2" \
                      || fail "12: both flags — version is 4.2" "got '$v'"
v=$(jhas /tmp/r12.json "VAST-2.0-url-empty")
[[ "$v" == "False" ]] && ok "12b: both flags — no url-empty error" \
                        || fail "12b: both flags — no url-empty error" "got '$v'"

# ─── 13. baseline: valid 4.2 fixture unchanged (no flags) ────────────────────
$BIN check crates/vastlint-core/tests/fixtures/valid_4.2.xml > /dev/null 2>&1 && ec=0 || ec=$?
[[ $ec -eq 0 ]] && ok "13: baseline valid_4.2.xml exits zero" \
                  || fail "13: baseline valid_4.2.xml exits zero" "exit was $ec"

echo ""
echo "════════════════════════════════════════"
echo " Results: $PASS passed, $FAIL failed"
echo "════════════════════════════════════════"
[[ $FAIL -eq 0 ]] && exit 0 || exit 1
