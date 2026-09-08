import { useCallback, useEffect, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import type { CommunityNodeConsentDocumentRef } from '@/lib/api';
import type { CommunityNodeConsentView, CommunityNodeEntryView } from '@/components/settings/types';

export type FetchCommunityNodePolicyView = (
  baseUrl: string, language?: string
) => void | Promise<CommunityNodeConsentView | void>;
export type AcceptCommunityNodePolicyView = (
  baseUrl: string, documents: CommunityNodeConsentDocumentRef[], language?: string
) => void | Promise<void>;

// Settings/Domeの既存callback契約を使い、共有cacheから表示snapshotを分離する。
export function useCommunityNodePolicyDialog({
  nodes, language, fetchPolicies, acceptPolicies, withdraw, onAccepted,
}: {
  nodes: readonly CommunityNodeEntryView[];
  language: string;
  fetchPolicies: FetchCommunityNodePolicyView;
  acceptPolicies: AcceptCommunityNodePolicyView;
  withdraw?: (baseUrl: string) => void | Promise<void>;
  onAccepted?: (node: CommunityNodeEntryView) => Promise<void>;
}) {
  const { t } = useTranslation('settings');
  const [target, setTarget] = useState<CommunityNodeEntryView | null>(null);
  const [entry, setEntry] = useState<{ language: string; view: CommunityNodeConsentView } | null>(null);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [attempt, setAttempt] = useState(0);
  const generation = useRef(0);
  const accepting = useRef(false);
  const returnFocus = useRef<HTMLElement | null>(null);
  const fetchRef = useRef(fetchPolicies);
  useEffect(() => { fetchRef.current = fetchPolicies; }, [fetchPolicies]);
  const configured = Boolean(target && nodes.some((node) => node.saved && node.baseUrl === target.baseUrl));

  useEffect(() => {
    if (!target || !configured) return;
    let active = true;
    const id = ++generation.current;
    setEntry(null);
    setError(null);
    setLoadError(null);
    void Promise.resolve().then(() => fetchRef.current(target.baseUrl, language)).then((view) => {
      if (active && generation.current === id) setEntry({
        language,
        // voidは既存の表示fixture用。production actionは取得responseからviewを返す。
        view: view ?? target.consent,
      });
    }).catch((cause: unknown) => {
      if (active && generation.current === id) setLoadError(cause instanceof Error ? cause.message : String(cause));
    });
    return () => { active = false; generation.current += 1; };
  }, [attempt, configured, language, target]);

  const close = useCallback(() => {
    if (accepting.current) return;
    generation.current += 1;
    setTarget(null);
    setEntry(null);
    setError(null);
  }, []);
  useEffect(() => { if (target && !configured) close(); }, [busy, close, configured, target]);

  function open(baseUrl: string) {
    if (accepting.current) return;
    const node = nodes.find((node) => node.saved && node.baseUrl === baseUrl);
    if (!node) return;
    returnFocus.current = document.activeElement instanceof HTMLElement ? document.activeElement : null;
    generation.current += 1;
    setTarget(node);
    setEntry(null);
    setAttempt((value) => value + 1);
  }
  const currentEntry = entry?.language === language ? entry : null;
  async function accept() {
    if (!target || !configured || !currentEntry?.view.loaded || accepting.current) return;
    const documents = currentEntry.view.policies.map((policy) => ({
      policy_slug: policy.policySlug, policy_version: policy.policyVersion,
      policy_snapshot_revision: policy.policySnapshotRevision ?? null,
    }));
    if (!documents.length || currentEntry.view.allRequiredAccepted) return;
    const id = generation.current;
    let released = false;
    accepting.current = true;
    setBusy(true);
    setError(null);
    try {
      await acceptPolicies(target.baseUrl, documents, currentEntry.language);
      if (id === generation.current) {
        setTarget(null);
        setEntry(null);
        accepting.current = false;
        setBusy(false);
        released = true;
        await onAccepted?.(target);
      }
    } catch {
      if (id === generation.current) setError(t('communityNode.consent.acceptFailed'));
    } finally {
      if (!released) {
        accepting.current = false;
        setBusy(false);
      }
    }
  }
  async function withdrawConsent() {
    if (!target || !configured || !withdraw || accepting.current) return;
    const id = generation.current;
    accepting.current = true;
    setBusy(true);
    setError(null);
    try {
      await withdraw(target.baseUrl);
      if (id === generation.current) { setTarget(null); setEntry(null); }
    } catch {
      if (id === generation.current) setError(t('communityNode.consent.acceptFailed'));
    } finally {
      accepting.current = false;
      setBusy(false);
    }
  }
  return {
    open,
    dialog: target && configured ? {
      open: true, baseUrl: target.baseUrl,
      consent: currentEntry?.view ?? {
        ...target.consent, loaded: false, loading: !loadError, loadError, policies: [],
      },
      busy, error,
      onOpenChange: (next: boolean) => { if (!next) close(); },
      onAccept: () => { void accept(); },
      onRetry: () => { if (!accepting.current) setAttempt((value) => value + 1); },
      onWithdraw: withdraw ? () => { void withdrawConsent(); } : undefined,
      onCloseAutoFocus: (event: Event) => { event.preventDefault(); if (returnFocus.current?.isConnected) returnFocus.current.focus(); },
    } : null,
  };
}
