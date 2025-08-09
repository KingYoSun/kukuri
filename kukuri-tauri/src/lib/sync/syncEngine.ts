import type { OfflineAction } from '@/types/offline';
import { OfflineActionType } from '@/types/offline';
import { TauriApi } from '@/lib/api/tauri';
import { p2pApi } from '@/lib/api/p2p';
import { subscribeToTopic as nostrSubscribe } from '@/lib/api/nostr';

export interface SyncConflict {
  localAction: OfflineAction;
  remoteAction?: OfflineAction;
  conflictType: 'timestamp' | 'version' | 'merge';
  resolution?: 'local' | 'remote' | 'merge' | 'manual';
  mergedData?: any;
}

export interface SyncResult {
  syncedActions: OfflineAction[];
  conflicts: SyncConflict[];
  failedActions: OfflineAction[];
  totalProcessed: number;
}

export interface DiffPatch {
  type: 'add' | 'modify' | 'delete';
  path: string;
  value?: any;
  oldValue?: any;
}

export class SyncEngine {
  private api: TauriApi;
  private isSyncing = false;
  private syncQueue: Map<string, OfflineAction[]> = new Map();
  
  constructor() {
    this.api = new TauriApi();
  }

  /**
   * 差分同期アルゴリズムの実装
   */
  async performDifferentialSync(
    localActions: OfflineAction[], 
    remoteCheckpoint?: string
  ): Promise<SyncResult> {
    const result: SyncResult = {
      syncedActions: [],
      conflicts: [],
      failedActions: [],
      totalProcessed: 0,
    };

    if (this.isSyncing) {
      throw new Error('同期処理が既に実行中です');
    }

    this.isSyncing = true;

    try {
      // トピック別にアクションをグループ化
      const groupedActions = this.groupActionsByTopic(localActions);
      
      // 並列同期処理
      const syncPromises = Array.from(groupedActions.entries()).map(
        ([topicId, actions]) => this.syncTopicActions(topicId, actions)
      );
      
      const topicResults = await Promise.allSettled(syncPromises);
      
      // 結果を集約
      for (const topicResult of topicResults) {
        if (topicResult.status === 'fulfilled') {
          result.syncedActions.push(...topicResult.value.syncedActions);
          result.conflicts.push(...topicResult.value.conflicts);
          result.failedActions.push(...topicResult.value.failedActions);
          result.totalProcessed += topicResult.value.totalProcessed;
        } else {
          console.error('トピック同期エラー:', topicResult.reason);
        }
      }
      
      return result;
    } finally {
      this.isSyncing = false;
    }
  }

  /**
   * トピック別にアクションをグループ化
   */
  private groupActionsByTopic(actions: OfflineAction[]): Map<string, OfflineAction[]> {
    const grouped = new Map<string, OfflineAction[]>();
    
    for (const action of actions) {
      const topicId = action.actionData?.topicId || 'default';
      if (!grouped.has(topicId)) {
        grouped.set(topicId, []);
      }
      grouped.get(topicId)!.push(action);
    }
    
    return grouped;
  }

  /**
   * トピック単位でアクションを同期
   */
  private async syncTopicActions(
    topicId: string, 
    actions: OfflineAction[]
  ): Promise<SyncResult> {
    const result: SyncResult = {
      syncedActions: [],
      conflicts: [],
      failedActions: [],
      totalProcessed: 0,
    };

    for (const action of actions) {
      try {
        result.totalProcessed++;
        
        // タイムスタンプベースの競合検出
        const conflict = await this.detectConflict(action);
        
        if (conflict) {
          // 競合解決
          const resolved = await this.resolveConflict(conflict);
          if (resolved.resolution === 'local' || resolved.resolution === 'merge') {
            await this.applyAction(action);
            result.syncedActions.push(action);
          }
          result.conflicts.push(resolved);
        } else {
          // 競合なし - アクションを適用
          await this.applyAction(action);
          result.syncedActions.push(action);
        }
      } catch (error) {
        console.error(`アクション同期エラー (${action.localId}):`, error);
        result.failedActions.push(action);
      }
    }
    
    return result;
  }

  /**
   * タイムスタンプベースの競合検出
   */
  async detectConflict(action: OfflineAction): Promise<SyncConflict | null> {
    // エンティティの最終更新時刻を取得
    const lastModified = await this.getEntityLastModified(
      action.actionData?.entityType, 
      action.actionData?.entityId
    );
    
    if (!lastModified) {
      return null;
    }
    
    // アクションの作成時刻と比較
    const actionTimestamp = new Date(action.createdAt).getTime();
    const entityTimestamp = new Date(lastModified).getTime();
    
    if (entityTimestamp > actionTimestamp) {
      // 競合検出
      return {
        localAction: action,
        conflictType: 'timestamp',
      };
    }
    
    return null;
  }

  /**
   * 競合解決ロジック
   */
  async resolveConflict(conflict: SyncConflict): Promise<SyncConflict> {
    switch (conflict.conflictType) {
      case 'timestamp':
        // Last-Write-Wins (LWW) ベースライン実装
        return this.resolveLWW(conflict);
        
      case 'version':
        // バージョンベースの解決
        return this.resolveVersionConflict(conflict);
        
      case 'merge':
        // カスタムマージルール
        return this.applyCustomMergeRules(conflict);
        
      default:
        // デフォルトはLWW
        return this.resolveLWW(conflict);
    }
  }

  /**
   * Last-Write-Wins (LWW) 競合解決
   */
  private resolveLWW(conflict: SyncConflict): SyncConflict {
    const localTime = new Date(conflict.localAction.createdAt).getTime();
    const remoteTime = conflict.remoteAction 
      ? new Date(conflict.remoteAction.createdAt).getTime() 
      : 0;
    
    if (localTime >= remoteTime) {
      conflict.resolution = 'local';
    } else {
      conflict.resolution = 'remote';
    }
    
    return conflict;
  }

  /**
   * バージョンベースの競合解決
   */
  private resolveVersionConflict(conflict: SyncConflict): SyncConflict {
    // バージョン番号の比較ロジック
    const localVersion = conflict.localAction.actionData?.version || 0;
    const remoteVersion = conflict.remoteAction?.actionData?.version || 0;
    
    if (localVersion >= remoteVersion) {
      conflict.resolution = 'local';
    } else {
      conflict.resolution = 'remote';
    }
    
    return conflict;
  }

  /**
   * カスタムマージルールの適用
   */
  private applyCustomMergeRules(conflict: SyncConflict): SyncConflict {
    const actionType = conflict.localAction.actionType;
    
    switch (actionType) {
      case OfflineActionType.JOIN_TOPIC:
      case OfflineActionType.LEAVE_TOPIC:
        // トピック参加状態は最新の状態を優先
        return this.resolveLWW(conflict);
        
      case OfflineActionType.CREATE_POST:
        // 投稿は両方を保持（重複の可能性あり）
        conflict.resolution = 'local';
        break;
        
      case OfflineActionType.LIKE_POST:
        // いいねは重複しても問題ない
        conflict.resolution = 'local';
        break;
        
      default:
        // デフォルトはLWW
        return this.resolveLWW(conflict);
    }
    
    return conflict;
  }

  /**
   * アクションを適用
   */
  private async applyAction(action: OfflineAction): Promise<void> {
    switch (action.actionType) {
      case OfflineActionType.CREATE_POST:
        await this.api.createPost(
          action.actionData.content,
          action.actionData.topicId,
          action.actionData.replyTo,
          action.actionData.quotedPost
        );
        break;
        
      case OfflineActionType.LIKE_POST:
        await this.api.likePost(action.actionData.postId);
        break;
        
      case OfflineActionType.JOIN_TOPIC:
        await p2pApi.joinTopic(action.actionData.topicId);
        await nostrSubscribe(action.actionData.topicId);
        break;
        
      case OfflineActionType.LEAVE_TOPIC:
        await p2pApi.leaveTopic(action.actionData.topicId);
        break;
        
      default:
        throw new Error(`未対応のアクションタイプ: ${action.actionType}`);
    }
  }

  /**
   * エンティティの最終更新時刻を取得
   */
  private async getEntityLastModified(
    entityType?: string, 
    entityId?: string
  ): Promise<string | null> {
    if (!entityType || !entityId) {
      return null;
    }
    
    try {
      // エンティティタイプに応じてメタデータを取得
      switch (entityType) {
        case 'post': {
          // 投稿のメタデータを取得
          const { invoke } = await import('@tauri-apps/api/core');
          const result = await invoke<{ updated_at: string }>('get_post_metadata', {
            postId: entityId
          }).catch(() => null);
          return result?.updated_at || null;
        }
        
        case 'topic': {
          // トピックのメタデータを取得
          const { invoke } = await import('@tauri-apps/api/core');
          const result = await invoke<{ updated_at: string }>('get_topic_metadata', {
            topicId: entityId
          }).catch(() => null);
          return result?.updated_at || null;
        }
        
        case 'user': {
          // ユーザーのメタデータを取得
          const { invoke } = await import('@tauri-apps/api/core');
          const result = await invoke<{ updated_at: string }>('get_user_metadata', {
            userId: entityId
          }).catch(() => null);
          return result?.updated_at || null;
        }
        
        case 'reaction': {
          // リアクションのメタデータを取得
          const { invoke } = await import('@tauri-apps/api/core');
          const result = await invoke<{ created_at: string }>('get_reaction_metadata', {
            reactionId: entityId
          }).catch(() => null);
          return result?.created_at || null;
        }
        
        default:
          // その他のエンティティタイプ
          console.warn(`未対応のエンティティタイプ: ${entityType}`);
          return null;
      }
    } catch (error) {
      console.error('エンティティメタデータ取得エラー:', error);
      return null;
    }
  }

  /**
   * 差分パッチの生成
   */
  generateDiffPatches(oldData: any, newData: any): DiffPatch[] {
    const patches: DiffPatch[] = [];
    
    // 簡単な差分検出実装
    const processObject = (old: any, current: any, path = '') => {
      // 新規追加
      for (const key in current) {
        const currentPath = path ? `${path}.${key}` : key;
        
        if (!(key in old)) {
          patches.push({
            type: 'add',
            path: currentPath,
            value: current[key],
          });
        } else if (JSON.stringify(old[key]) !== JSON.stringify(current[key])) {
          // 変更
          patches.push({
            type: 'modify',
            path: currentPath,
            value: current[key],
            oldValue: old[key],
          });
        }
      }
      
      // 削除
      for (const key in old) {
        const currentPath = path ? `${path}.${key}` : key;
        
        if (!(key in current)) {
          patches.push({
            type: 'delete',
            path: currentPath,
            oldValue: old[key],
          });
        }
      }
    };
    
    processObject(oldData || {}, newData || {});
    
    return patches;
  }

  /**
   * 差分パッチの適用
   */
  applyDiffPatches(data: any, patches: DiffPatch[]): any {
    const result = { ...data };
    
    for (const patch of patches) {
      const keys = patch.path.split('.');
      let target = result;
      
      // パスを辿る
      for (let i = 0; i < keys.length - 1; i++) {
        if (!target[keys[i]]) {
          target[keys[i]] = {};
        }
        target = target[keys[i]];
      }
      
      const lastKey = keys[keys.length - 1];
      
      switch (patch.type) {
        case 'add':
        case 'modify':
          target[lastKey] = patch.value;
          break;
          
        case 'delete':
          delete target[lastKey];
          break;
      }
    }
    
    return result;
  }
}

// シングルトンインスタンス
export const syncEngine = new SyncEngine();