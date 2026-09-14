"""Run with Linux/WSL Python: python3 -m unittest discover -s infra/terraform/tests."""
import os
from pathlib import Path
import subprocess
import unittest

HELPER = Path(__file__).resolve().parents[1] / 'modules/gcp-vm-compose/scripts/index-health.sh'


class IndexHealthTests(unittest.TestCase):
    def metrics(self, topics=('kukuri:topic:general', 'kukuri:topic:test'), db='2|3',
                db_error=False, logs='', log_error=False):
        env = dict(os.environ, MOCK_DB=db, MOCK_LOGS=logs,
                   DB_ERROR=str(int(db_error)), LOG_ERROR=str(int(log_error)))
        script = '''
set -euo pipefail
docker() {
  if [ "$1" = exec ]; then
    [ "$DB_ERROR" = 0 ] || return 1
    printf '%s\\n' "$MOCK_DB"
  elif [ "$1" = logs ]; then
    [ "$LOG_ERROR" = 0 ] || return 1
    printf '%s\\n' "$MOCK_LOGS"
  else return 99; fi
}
source "$1"
shift
index_health_metrics postgres indexer "$@"
'''
        output = subprocess.run(['bash', '-c', script, 'test', str(HELPER), *topics],
                                env=env, text=True, capture_output=True, check=True).stdout
        return dict((key, int(value)) for key, value in (line.split() for line in output.splitlines()))

    def test_missing_starter_topics_are_unhealthy(self):
        self.assertEqual(self.metrics(db='0|0')['index_expected_topics_present'], 0)

    def test_empty_index_is_observable_even_when_topics_exist(self):
        result = self.metrics(db='2|0')
        self.assertEqual(result['index_expected_topics_present'], 1)
        self.assertEqual(result['index_expected_topics_entries'], 0)

    def test_healthy_index_and_body_failures_are_separate(self):
        result = self.metrics(logs='failed to resolve post body; not indexing the post (fail-closed) secret\nother\nfailed to resolve post body; not indexing the post (fail-closed)')
        self.assertEqual(result, {'index_expected_topics_present': 1,
                                 'index_expected_topics_entries': 3,
                                 'body_fetch_failures_recent': 2})

    def test_db_failure_or_malformed_output_is_not_healthy(self):
        for kwargs in ({'db_error': True}, {'db': ''}, {'db': 'garbage'}, {'db': '2|3\n4|5'}):
            with self.subTest(kwargs=kwargs):
                result = self.metrics(**kwargs)
                self.assertEqual(result['index_expected_topics_present'], 0)
                self.assertEqual(result['index_expected_topics_entries'], -1)

    def test_log_failure_is_unknown(self):
        self.assertEqual(self.metrics(log_error=True)['body_fetch_failures_recent'], -1)

    def test_generic_empty_node_has_no_expected_topic_metrics(self):
        self.assertEqual(self.metrics(topics=(), db_error=True), {'body_fetch_failures_recent': 0})

    def test_topic_cannot_inject_sql_or_shell(self):
        with self.assertRaises(subprocess.CalledProcessError):
            self.metrics(topics=("x'); DELETE FROM cn_index.supported_topics; --",))


if __name__ == '__main__':
    unittest.main()
