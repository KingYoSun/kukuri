import json
import pathlib
import subprocess
import tempfile
import unittest
import xml.etree.ElementTree as ET


ROOT = pathlib.Path(__file__).resolve().parents[2]
MANIFEST = ROOT / "apps/desktop/src-tauri/windows/store/Package.appxmanifest"
SCRIPT = ROOT / "scripts/release/build-windows-store-msix.ps1"
WORKFLOW = ROOT / ".github/workflows/kukuri-windows-store-package.yml"
NS = {
    "f": "http://schemas.microsoft.com/appx/manifest/foundation/windows10",
    "uap": "http://schemas.microsoft.com/appx/manifest/uap/windows10",
    "rescap": "http://schemas.microsoft.com/appx/manifest/foundation/windows10/restrictedcapabilities",
    "uap10": "http://schemas.microsoft.com/appx/manifest/uap/windows10/10",
}


class WindowsStorePackageContracts(unittest.TestCase):
    def test_output_guard_rejects_existing_directories_without_deleting_content(self):
        # Execute only the real guard, never the packaging/deletion entrypoint.
        with tempfile.TemporaryDirectory() as directory:
            root = pathlib.Path(directory)
            existing = root / "dist" / "existing"
            existing.mkdir(parents=True)
            marker = existing / "keep.txt"
            marker.write_text("keep", encoding="utf-8")
            command = r'''
param($scriptPath, $repoRoot, $existing)
$ast = [Management.Automation.Language.Parser]::ParseFile($scriptPath, [ref]$null, [ref]$null)
$function = $ast.Find({param($node) $node -is [Management.Automation.Language.FunctionDefinitionAst] -and $node.Name -eq 'Assert-WorkspaceChild'}, $true)
. ([scriptblock]::Create($function.Extent.Text))
Assert-WorkspaceChild (Join-Path $repoRoot 'dist/new') 'OutputDirectory'
try { Assert-WorkspaceChild (Join-Path $repoRoot '.git/new') 'OutputDirectory'; exit 2 } catch {}
try { Assert-WorkspaceChild $existing 'OutputDirectory' } catch { exit 0 }
exit 1
'''
            test_script = root / "guard-test.ps1"
            test_script.write_text(command, encoding="utf-8")
            result = subprocess.run(["pwsh", "-NoProfile", "-File", str(test_script), str(SCRIPT), str(root), str(existing)], capture_output=True, text=True)
            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
            self.assertEqual(marker.read_text(encoding="utf-8"), "keep")

    def test_manifest_matches_the_registered_partner_center_identity(self):
        root = ET.parse(MANIFEST).getroot()
        identity = root.find("f:Identity", NS)
        self.assertIsNotNone(identity)
        self.assertEqual(identity.attrib, {
            "Name": "KingYoSun.kukuri",
            "Publisher": "CN=33EB763C-4859-4E44-886F-1784E16DD6D5",
            "Version": "1.0.0.0",
            "ProcessorArchitecture": "x64",
        })
        properties = root.find("f:Properties", NS)
        self.assertEqual(properties.findtext("f:PublisherDisplayName", namespaces=NS), "KingYoSun")
        integrity = properties.find("uap10:PackageIntegrity/uap10:Content", NS)
        self.assertEqual(integrity.attrib["Enforcement"], "on")

    def test_build_bypass_is_rejected_before_any_tool_or_output(self):
        result = subprocess.run(["pwsh", "-NoProfile", "-File", str(SCRIPT), "-SkipBuild"], capture_output=True, text=True)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("SkipBuild", result.stderr)
        source = SCRIPT.read_text(encoding="utf-8")
        self.assertIn('$env:CARGO_TARGET_DIR = $storeTargetDir', source)
        self.assertNotIn('Remove-Item', source)

    def test_manifest_exposes_only_the_required_full_trust_surface(self):
        root = ET.parse(MANIFEST).getroot()
        application = root.find("f:Applications/f:Application", NS)
        self.assertEqual(application.attrib["Executable"], "kukuri.exe")
        self.assertEqual(application.attrib["EntryPoint"], "Windows.FullTrustApplication")
        self.assertEqual(application.attrib[f"{{{NS['uap10']}}}TrustLevel"], "mediumIL")
        self.assertEqual(application.attrib[f"{{{NS['uap10']}}}RuntimeBehavior"], "packagedClassicApp")
        protocol = application.find("f:Extensions/uap:Extension/uap:Protocol", NS)
        self.assertEqual(protocol.attrib["Name"], "kukuri")
        capabilities = root.findall("f:Capabilities/*", NS)
        self.assertEqual(len(capabilities), 1)
        self.assertEqual(capabilities[0].tag, f"{{{NS['rescap']}}}Capability")
        self.assertEqual(capabilities[0].attrib["Name"], "runFullTrust")
        self.assertNotRegex(MANIFEST.read_text(encoding="utf-8"), r"\$[A-Za-z].*?\$")

    def test_packaging_produces_only_the_unsigned_store_candidate(self):
        source = SCRIPT.read_text(encoding="utf-8")
        self.assertIn('$requiredWinAppVersion = "0.6.1"', source)
        self.assertRegex(source, r'"pack", \$stagingDir,\s*"--manifest"')
        self.assertNotIn('"--cert"', source)
        self.assertNotIn('"--cert-password"', source)
        self.assertNotIn("code_sign_certificate", source)
        self.assertNotIn("KUKURI_MSIX_CERT_PASSWORD", source)
        self.assertNotIn("SignTool", source)
        self.assertNotIn("Import-PfxCertificate", source)
        self.assertNotIn("Import-Certificate", source)
        self.assertIn('"--features", "microsoft-store"', source)
        self.assertIn('$env:VITE_KUKURI_DISTRIBUTION = "microsoft-store"', source)
        self.assertIn("MSIX payload does not match the fixed allowlist", source)
        self.assertIn("MSIX block map must use SHA-256", source)

    def test_ci_builds_an_unsigned_package_without_distribution_secrets(self):
        source = WORKFLOW.read_text(encoding="utf-8")
        self.assertNotIn("secrets.", source)
        self.assertIn(
            "uses: microsoft/setup-WinAppCli@cc8ea9a08b3ee3db43d5aa6bddda4a0e87d800f7",
            source,
        )
        self.assertIn("version: v0.6.1", source)
        self.assertIn("Expected WinApp CLI 0.6.1", source)
        self.assertIn("run: cargo xtask windows-store-package", source)
        self.assertNotIn("--sign-local", source)


if __name__ == "__main__":
    unittest.main()
