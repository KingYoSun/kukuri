import { useMutation, useQueryClient } from '@tanstack/react-query'
import { useAuthStore } from '@/stores'
import type { User } from '@/stores'

// 仮のAPI関数（後でTauriコマンドに置き換え）
const loginWithPrivateKey = async (privateKey: string): Promise<{ user: User; privateKey: string }> => {
  // TODO: Tauriバックエンドで鍵の検証とユーザー情報の取得
  const user: User = {
    id: 'user' + privateKey.slice(0, 8),
    pubkey: 'pubkey' + privateKey.slice(0, 8),
    npub: 'npub' + privateKey.slice(0, 8),
    name: 'テストユーザー',
    displayName: 'テストユーザー',
    picture: '',
    about: 'kukuriのテストユーザーです',
    nip05: ''
  }
  return { user, privateKey }
}

const generateNewKeyPair = async (): Promise<{ user: User; privateKey: string }> => {
  // TODO: Tauriバックエンドで新しい鍵ペアを生成
  const privateKey = 'nsec' + Math.random().toString(36).substring(2, 15)
  const id = Math.random().toString(36).substring(2, 10)
  const user: User = {
    id: 'user' + id,
    pubkey: 'pubkey' + id,
    npub: 'npub' + id,
    name: '',
    displayName: '',
    picture: '',
    about: '',
    nip05: ''
  }
  return { user, privateKey }
}

export const useLogin = () => {
  const queryClient = useQueryClient()
  const { login } = useAuthStore()

  return useMutation({
    mutationFn: loginWithPrivateKey,
    onSuccess: ({ user, privateKey }) => {
      login(privateKey, user)
      queryClient.invalidateQueries()
    },
  })
}

export const useGenerateKeyPair = () => {
  const queryClient = useQueryClient()
  const { login } = useAuthStore()

  return useMutation({
    mutationFn: generateNewKeyPair,
    onSuccess: ({ user, privateKey }) => {
      login(privateKey, user)
      queryClient.invalidateQueries()
    },
  })
}

export const useLogout = () => {
  const queryClient = useQueryClient()
  const { logout } = useAuthStore()

  return () => {
    logout()
    queryClient.clear()
  }
}