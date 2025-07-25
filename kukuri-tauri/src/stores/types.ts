export interface User {
  pubkey: string
  name?: string
  picture?: string
  about?: string
  nip05?: string
  created_at: number
}

export interface Topic {
  id: string
  name: string
  description?: string
  tags: string[]
  memberCount: number
  lastActive: number
}

export interface Post {
  id: string
  pubkey: string
  content: string
  topicId: string
  created_at: number
  tags: string[][]
  replies?: Post[]
}

export interface AuthState {
  isAuthenticated: boolean
  currentUser: User | null
  privateKey: string | null
}

export interface TopicState {
  topics: Map<string, Topic>
  currentTopic: Topic | null
  joinedTopics: string[]
}

export interface PostState {
  posts: Map<string, Post>
  postsByTopic: Map<string, string[]>
}