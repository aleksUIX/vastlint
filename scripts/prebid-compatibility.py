#!/usr/bin/env python3
"""Run the pinned PBS Go client against the actual Rust binary or a digest-pinned image."""
import argparse
import json
import os
from pathlib import Path
import re
import socket
import subprocess
import time
import urllib.request


def check_proto(public, subset):
    def messages(path):
        text = re.sub(r'//[^\n]*', '', path.read_text())
        return {name: {int(number): (kind, field) for kind, field, number in re.findall(r'\b([\w.]+)\s+(\w+)\s*=\s*(\d+)\s*;', body)}
                for name, body in re.findall(r'message\s+(\w+)\s*\{([^}]+)\}', text)}
    full, small = messages(public), messages(subset)
    for name, fields in small.items():
        for number, signature in fields.items():
            if full.get(name, {}).get(number) != signature:
                raise RuntimeError(f'protobuf contract mismatch: {name} field {number}: {signature}')
    for field in range(1, 6):
        if field not in small['ValidateResponse']:
            raise RuntimeError(f'missing monitoring response field {field}')


def free_port():
    with socket.socket() as sock:
        sock.bind(('127.0.0.1', 0))
        return sock.getsockname()[1]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--pbs-path', type=Path, required=True)
    parser.add_argument('--work-dir', type=Path, required=True)
    server = parser.add_mutually_exclusive_group(required=True)
    server.add_argument('--validator-bin', type=Path)
    server.add_argument('--image')
    args = parser.parse_args()
    root = Path(__file__).resolve().parent.parent
    args.work_dir.mkdir(parents=True, exist_ok=True)
    check_proto(root / 'proto/openadtech/vastlint/v1/vastlint.proto', args.pbs_path / 'analytics/vastlint/api/vastlint.proto')
    if args.image and not re.fullmatch(r'[\w./-]+@sha256:[0-9a-f]{64}', args.image):
        parser.error('--image must be pinned by sha256 digest')
    binary = args.work_dir / 'pbs-vastlint-compatibility.test'
    subprocess.run(['go', 'test', '-c', '-tags', 'vastlint_integration', '-o', str(binary), './analytics/vastlint'], cwd=args.pbs_path, check=True)
    results = []
    for level in ('default', 'off', 'info', 'warning', 'error'):
        policy = {'profile': f'prebid-{level}', 'rules': {}}
        if level != 'default':
            policy['rules']['VAST-2.0-inline-impression'] = level
        rules = args.work_dir / f'rules-{level}.json'
        rules.write_text(json.dumps(policy))
        grpc_port, metrics_port = free_port(), free_port()
        while grpc_port == metrics_port:
            metrics_port = free_port()
        env = os.environ | {'VASTLINT_RULES_FILE': str(rules), 'VASTLINT_GRPC_ADDR': f'127.0.0.1:{grpc_port}', 'VASTLINT_METRICS_ADDR': f'127.0.0.1:{metrics_port}', 'VASTLINT_BLOCKING_THREADS': '2', 'VASTLINT_WORKER_THREADS': '2', 'VASTLINT_MAX_MESSAGE_BYTES': '132096'}
        name = f'vastlint-compatibility-{os.getpid()}-{level}'
        if args.image:
            command = ['docker', 'run', '--rm', '--name', name, '--read-only', '--cpus', '2', '--memory', '256m', '--pids-limit', '64', '--cap-drop=ALL', '--security-opt=no-new-privileges', '-p', f'127.0.0.1:{grpc_port}:50051', '-p', f'127.0.0.1:{metrics_port}:9090', '-v', f'{rules.resolve()}:/rules.json:ro', '-e', 'VASTLINT_RULES_FILE=/rules.json', '-e', 'VASTLINT_BLOCKING_THREADS=2', '-e', 'VASTLINT_WORKER_THREADS=2', '-e', 'VASTLINT_MAX_MESSAGE_BYTES=132096', args.image]
        else:
            command = [str(args.validator_bin.resolve())]
        with (args.work_dir / f'server-{level}.log').open('w') as log:
            process = subprocess.Popen(command, env=env, stdout=log, stderr=log)
            try:
                url = f'http://127.0.0.1:{metrics_port}/metrics'
                for _ in range(100):
                    if process.poll() is not None:
                        raise RuntimeError(f'validator exited: inspect server-{level}.log')
                    try:
                        with urllib.request.urlopen(url, timeout=.2) as response:
                            if response.status == 200:
                                with socket.create_connection(('127.0.0.1', grpc_port), timeout=.2):
                                    break
                    except (OSError, TimeoutError):
                        time.sleep(.1)
                else:
                    raise RuntimeError('validator metrics did not become ready')
                test_env = os.environ | {'VASTLINT_TEST_ENDPOINT': f'127.0.0.1:{grpc_port}', 'VASTLINT_TEST_METRICS_URL': url, 'VASTLINT_TEST_RULE_LEVEL': level}
                match = 'TestRust' if level == 'default' else 'TestRustHostRuleLevels'
                with (args.work_dir / f'client-{level}.log').open('w') as output:
                    subprocess.run([str(binary), '-test.v', '-test.run', match], env=test_env, stdout=output, stderr=subprocess.STDOUT, check=True, timeout=90)
                with urllib.request.urlopen(url, timeout=2) as response:
                    (args.work_dir / f'metrics-{level}.txt').write_bytes(response.read())
                results.append({'level': level, 'passed': True})
                print(f'{level}: passed', flush=True)
            finally:
                if args.image:
                    subprocess.run(['docker', 'stop', '-t', '3', name], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, check=False)
                process.terminate()
                try:
                    process.wait(timeout=5)
                except subprocess.TimeoutExpired:
                    process.kill()
                    process.wait()
    (args.work_dir / 'results.json').write_text(json.dumps({'server': args.image or str(args.validator_bin), 'profiles': results}, indent=2) + '\n')


if __name__ == '__main__':
    main()
