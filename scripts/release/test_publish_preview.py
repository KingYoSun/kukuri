import json
import pathlib
import sys
import tempfile
import unittest
from unittest.mock import patch

sys.path.insert(0, str(pathlib.Path(__file__).parent))
import publish_preview as publisher


class PublishTests(unittest.TestCase):
    def test_incomplete_input_or_changed_tag_never_creates_a_release(self):
        for changed_tag in (False, True):
            calls = []

            def api(path, method="GET", data=None, file=None):
                calls.append(method)
                return {"sha": "b" * 40}

            with patch.object(publisher, "validate_output", return_value=[] if changed_tag else None) as validate:
                if not changed_tag: validate.side_effect = ValueError("incomplete")
                with self.assertRaises(ValueError):
                    publisher.publish(pathlib.Path("unused"), "v0.1.8-preview.2", "KingYoSun/kukuri", "a" * 40, False, api)
            self.assertNotIn("POST", calls)
            self.assertNotIn("PATCH", calls)

    def test_upload_failure_cannot_publish_and_success_only_patches_new_id(self):
        for fail_upload in (False, True):
            with self.subTest(fail=fail_upload), tempfile.TemporaryDirectory() as work:
                root = pathlib.Path(work)
                (root / "asset").write_text("fixture")
                (root / "RELEASE_NOTES_DRAFT.md").write_text("fixture notes")
                calls = []

                def api(path, method="GET", data=None, file=None):
                    calls.append((path, method, data))
                    if method == "GET":
                        if "/releases/tags/" in path: raise publisher.GitHubError(404)
                        return {"sha": "a" * 40}
                    if file:
                        if fail_upload: raise RuntimeError("fixture upload failed")
                        return {"digest": "sha256:" + publisher.file_record(root, "asset")["sha256"]}
                    return {"id": 123}

                with patch.object(publisher, "validate_output", return_value=["asset"]):
                    if fail_upload:
                        with self.assertRaises(RuntimeError):
                            publisher.publish(root, "v0.1.8-preview.2", "KingYoSun/kukuri", "a" * 40, False, api)
                    else:
                        publisher.publish(root, "v0.1.8-preview.2", "KingYoSun/kukuri", "a" * 40, False, api)
                patches = [call for call in calls if call[1] == "PATCH"]
                self.assertEqual(len(patches), 0 if fail_upload else 1)
                if patches: self.assertEqual(patches[0][0], "repos/KingYoSun/kukuri/releases/123")
                self.assertTrue(next(call[2] for call in calls if call[1] == "POST")["draft"])
                self.assertFalse(next(call[2] for call in calls if call[1] == "POST")["prerelease"])
                if patches: self.assertEqual(patches[0][2]["make_latest"], "true")

    def test_existing_candidate_is_reused_without_replacing_assets(self):
        with tempfile.TemporaryDirectory() as work:
            root = pathlib.Path(work)
            (root / "asset").write_text("fixture")
            (root / "RELEASE_NOTES_DRAFT.md").write_text("notes")
            calls = []
            existing = {"id": 123, "draft": True, "target_commitish": "a" * 40, "body": "notes",
                        "assets": [{"name": "asset", "digest": "sha256:" + publisher.file_record(root, "asset")["sha256"]}]}

            def api(path, method="GET", data=None, file=None):
                calls.append(method)
                return existing if "/releases/tags/" in path else {"sha": "a" * 40}

            with patch.object(publisher, "validate_output", return_value=["asset"]):
                publisher.publish(root, "v0.1.8-preview.2", "KingYoSun/kukuri", "a" * 40, True, api)
                self.assertNotIn("POST", calls)
                self.assertNotIn("PATCH", calls)
                existing["assets"][0]["digest"] = "sha256:" + "b" * 64
                with self.assertRaises(ValueError):
                    publisher.publish(root, "v0.1.8-preview.2", "KingYoSun/kukuri", "a" * 40, False, api)
                self.assertNotIn("PATCH", calls)

    def test_existing_empty_draft_with_other_notes_is_not_modified(self):
        with tempfile.TemporaryDirectory() as work:
            root = pathlib.Path(work)
            (root / "asset").write_text("fixture")
            (root / "RELEASE_NOTES_DRAFT.md").write_text("expected notes")
            writes = []

            def api(path, method="GET", data=None, file=None):
                if method != "GET": writes.append(method)
                if "/releases/tags/" in path:
                    return {"id": 123, "draft": True, "target_commitish": "a" * 40, "body": "other notes", "assets": []}
                return {"sha": "a" * 40, "digest": "sha256:" + publisher.file_record(root, "asset")["sha256"]}

            with patch.object(publisher, "validate_output", return_value=["asset"]):
                with self.assertRaises(ValueError):
                    publisher.publish(root, "v0.1.8-preview.2", "KingYoSun/kukuri", "a" * 40, False, api)
            self.assertEqual(writes, [])


if __name__ == "__main__":
    unittest.main()
