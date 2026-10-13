#!/usr/bin/env python3
"""Measure synthetic complete HTTP auction paths. No bidder or production traffic."""
import argparse
import json
import os
from pathlib import Path
import platform
import random
import socket
import subprocess
import time
import urllib.request


def port():
    with socket.socket() as sock:
        sock.bind(('127.0.0.1', 0))
        return sock.getsockname()[1]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--endpoint-test-bin', type=Path, required=True)
    parser.add_argument('--validator-bin', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--requests', type=int, default=1000)
    parser.add_argument('--repeats', type=int, default=3)
    parser.add_argument('--scenarios', nargs='+', default=['disabled','healthy','unreachable','slow','saturated'])
    parser.add_argument('--traffic', nargs='+', default=['video','banner','excluded','large_user'])
    args = parser.parse_args()
    args.output.mkdir(parents=True, exist_ok=True)
    rules = args.output / 'rules.json'
    rules.write_text('{"profile":"load-default","rules":{}}\n')
    grpc, metrics = port(), port()
    while grpc == metrics:
        metrics = port()
    env = os.environ | {'VASTLINT_GRPC_ADDR': f'127.0.0.1:{grpc}', 'VASTLINT_METRICS_ADDR': f'127.0.0.1:{metrics}', 'VASTLINT_RULES_FILE': str(rules), 'VASTLINT_BLOCKING_THREADS': '2', 'VASTLINT_WORKER_THREADS': '2'}
    with (args.output / 'validator.log').open('w') as log:
        server = subprocess.Popen([str(args.validator_bin.resolve())], env=env, stdout=log, stderr=log)
        try:
            url = f'http://127.0.0.1:{metrics}/metrics'
            for _ in range(100):
                try:
                    with urllib.request.urlopen(url, timeout=.2):
                        break
                except OSError:
                    time.sleep(.1)
            else:
                raise RuntimeError('validator startup failed')
            cases = [(repeat, percent, scenario, traffic) for repeat in range(args.repeats) for percent in (1, 100) for scenario in args.scenarios for traffic in args.traffic]
            random.Random(20261012).shuffle(cases)
            results = []
            peak_validator_rss_kib = 0
            for i, (repeat, percent, scenario, traffic) in enumerate(cases):
                name = f'{repeat}-{percent}-{scenario}-{traffic}'
                result_path = args.output / f'{name}.json'
                test_env = os.environ | {'VASTLINT_LOAD_SCENARIO': scenario, 'VASTLINT_LOAD_TRAFFIC': traffic, 'VASTLINT_LOAD_REQUESTS': str(args.requests), 'VASTLINT_LOAD_SAMPLE_PERCENT': str(percent), 'VASTLINT_LOAD_RESULT': str(result_path), 'VASTLINT_TEST_ENDPOINT': f'127.0.0.1:{grpc}'}
                with (args.output / f'{name}.log').open('w') as output:
                    process = subprocess.Popen([str(args.endpoint_test_bin), '-test.run', '^TestVastlintOperational$', '-test.v', '-test.timeout=60s'], env=test_env, stdout=output, stderr=subprocess.STDOUT)
                    while process.poll() is None:
                        value = subprocess.check_output(['ps', '-o', 'rss=', '-p', str(server.pid)], text=True).strip()
                        peak_validator_rss_kib = max(peak_validator_rss_kib, int(value or '0'))
                        time.sleep(.1)
                    if process.returncode != 0:
                        raise RuntimeError(f'{name} failed, inspect its log')
                result = json.loads(result_path.read_text())
                result['repeat'] = repeat
                results.append(result)
                print(f'{i+1}/{len(cases)} {name}: zero response failures', flush=True)
            with urllib.request.urlopen(url, timeout=2) as response:
                (args.output / 'validator-metrics.txt').write_bytes(response.read())
            report = {'environment': {'platform': platform.platform(), 'go': subprocess.check_output(['go', 'version'], text=True).strip(), 'requests_per_run': args.requests, 'repeats': args.repeats, 'concurrency': 16, 'sample_percents': [1, 100]}, 'limitations': 'Synthetic local HTTP traffic with a fake auction exchange. HTTP client and PBS share a process; their CPU and allocations are combined. Healthy Rust validator is separate; slow/saturated fake validators share the test process. Peak RSS is per fresh test process including warmup. Validator RSS is sampled, not a guaranteed maximum. Not production host capacity or an unbiased traffic sample.', 'peak_sampled_validator_rss_bytes': peak_validator_rss_kib * 1024, 'runs': results}
            (args.output / 'results.json').write_text(json.dumps(report, indent=2) + '\n')
        finally:
            server.terminate()
            try:
                server.wait(timeout=5)
            except subprocess.TimeoutExpired:
                server.kill()
                server.wait()


if __name__ == '__main__':
    main()
