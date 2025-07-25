import { Card, CardContent, CardHeader } from "@/components/ui/card";
import { Avatar, AvatarFallback, AvatarImage } from "@/components/ui/avatar";
import { Button } from "@/components/ui/button";
import { Heart, MessageCircle, Repeat2, Share } from "lucide-react";

const posts = [
  {
    id: 1,
    author: {
      name: "Alice",
      avatar: "",
      pubkey: "npub1...",
    },
    content: "Nostrプロトコルを使った分散型SNSの可能性について考えています。中央集権的なプラットフォームからの脱却が実現できるかもしれません。",
    timestamp: "2時間前",
    likes: 12,
    comments: 3,
    reposts: 2,
  },
  {
    id: 2,
    author: {
      name: "Bob",
      avatar: "",
      pubkey: "npub2...",
    },
    content: "kukuriの開発進捗：P2P通信レイヤーの実装が完了しました！次はトピック機能の実装に取り掛かります。",
    timestamp: "4時間前",
    likes: 24,
    comments: 8,
    reposts: 5,
  },
];

export function Home() {
  return (
    <div className="max-w-2xl mx-auto">
      <h2 className="text-2xl font-bold mb-6">タイムライン</h2>
      
      <div className="space-y-4">
        {posts.map((post) => (
          <Card key={post.id} data-testid="card">
            <CardHeader>
              <div className="flex items-start gap-3">
                <Avatar>
                  <AvatarImage src={post.author.avatar} />
                  <AvatarFallback>{post.author.name[0]}</AvatarFallback>
                </Avatar>
                <div className="flex-1">
                  <div className="flex items-center gap-2">
                    <h4 className="font-semibold">{post.author.name}</h4>
                    <span className="text-sm text-muted-foreground">
                      {post.timestamp}
                    </span>
                  </div>
                  <p className="text-sm text-muted-foreground">
                    {post.author.pubkey}
                  </p>
                </div>
              </div>
            </CardHeader>
            <CardContent>
              <p className="mb-4">{post.content}</p>
              <div className="flex items-center gap-6">
                <Button variant="ghost" size="sm">
                  <MessageCircle className="mr-2 h-4 w-4" />
                  {post.comments}
                </Button>
                <Button variant="ghost" size="sm">
                  <Repeat2 className="mr-2 h-4 w-4" />
                  {post.reposts}
                </Button>
                <Button variant="ghost" size="sm">
                  <Heart className="mr-2 h-4 w-4" />
                  {post.likes}
                </Button>
                <Button variant="ghost" size="sm" aria-label="share">
                  <Share className="h-4 w-4" />
                </Button>
              </div>
            </CardContent>
          </Card>
        ))}
      </div>
    </div>
  );
}