import { Button } from "@/components/ui/button";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Separator } from "@/components/ui/separator";
import { Hash, Plus, TrendingUp, Users } from "lucide-react";

const topics = [
  { id: 1, name: "technology", icon: Hash, count: 1234 },
  { id: 2, name: "programming", icon: Hash, count: 892 },
  { id: 3, name: "nostr", icon: Hash, count: 456 },
  { id: 4, name: "bitcoin", icon: Hash, count: 789 },
];

const categories = [
  { name: "トレンド", icon: TrendingUp },
  { name: "フォロー中", icon: Users },
];

export function Sidebar() {
  return (
    <aside role="complementary" className="w-64 border-r bg-background">
      <div className="flex flex-col h-full">
        <div className="p-4">
          <Button className="w-full" variant="default">
            <Plus className="mr-2 h-4 w-4" />
            新規投稿
          </Button>
        </div>

        <Separator />

        <div className="p-4">
          <h3 className="mb-2 text-sm font-semibold text-muted-foreground">
            カテゴリー
          </h3>
          <div className="space-y-1">
            {categories.map((category) => (
              <Button
                key={category.name}
                variant="ghost"
                className="w-full justify-start"
              >
                <category.icon className="mr-2 h-4 w-4" />
                {category.name}
              </Button>
            ))}
          </div>
        </div>

        <Separator />

        <ScrollArea className="flex-1">
          <div className="p-4">
            <h3 className="mb-2 text-sm font-semibold text-muted-foreground">
              参加中のトピック
            </h3>
            <div className="space-y-1">
              {topics.map((topic) => (
                <Button
                  key={topic.id}
                  variant="ghost"
                  className="w-full justify-start"
                >
                  <topic.icon className="mr-2 h-4 w-4" />
                  <span className="flex-1 text-left">{topic.name}</span>
                  <span className="text-xs text-muted-foreground">
                    {topic.count}
                  </span>
                </Button>
              ))}
            </div>
          </div>
        </ScrollArea>
      </div>
    </aside>
  );
}