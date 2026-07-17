import urllib.request
import json


def get_digest(repo, tag):
    token_url = f'https://auth.docker.io/token?service=registry.docker.io&scope=repository:{repo}:pull'
    with urllib.request.urlopen(token_url) as r:
        token = json.load(r)['token']
    manifest_url = f'https://registry-1.docker.io/v2/{repo}/manifests/{tag}'
    req = urllib.request.Request(manifest_url, headers={
        'Authorization': f'Bearer {token}',
        'Accept': 'application/vnd.docker.distribution.manifest.v2+json'
    })
    with urllib.request.urlopen(req) as r:
        return r.headers['Docker-Content-Digest']

repos = [
    ('library/python', '3.12.7-slim'),
    ('library/node', '20.14.0-alpine3.20'),
    ('library/postgres', '16.4'),
    ('library/redis', '7.2.5'),
]
for repo, tag in repos:
    try:
        print(f'{repo}:{tag} => {get_digest(repo, tag)}')
    except Exception as e:
        print('ERROR', repo, tag, e)
